//! Frame generation's fast path (`FrameGeneration::Fast`): the made frame in one pass
//! (`reproject.wgsl`) from the depth, the camera's motion and the entities' own motion
//! vectors, for phones. FSR 3.1's optical flow and interpolation run some sixty passes at
//! the screen's size and build their bindings every frame: on a phone GPU that costs more
//! than drawing the frame, so turning it on lowered the frame rate.

use glam::Mat4;

/// `reproject.wgsl`'s `Params`.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    reproject: [[f32; 4]; 4],
    render_size: [f32; 2],
    jitter: [f32; 2],
}

/// The made frame's format, as the interpolation's (`Interpolation::output`).
const OUTPUT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

pub struct Inputs<'a> {
    /// This frame's world without the hand and HUD, at the screen's size; copied as the
    /// next frame's previous one, so it needs `COPY_SRC`.
    pub color: &'a wgpu::TextureView,
    /// This frame's depth at `render_size`: reversed, bound as unfilterable float.
    pub depth: &'a wgpu::TextureView,
    /// `motion::ObjectMotion::draw`'s vectors, at `render_size`.
    pub object_motion: &'a wgpu::TextureView,
    pub render_size: [u32; 2],
    /// TAA's sub-pixel jitter this frame, in render pixels.
    pub jitter: [f32; 2],
    /// This frame's unjittered clip space to the previous frame's.
    pub reproject: Mat4,
    /// No previous frame to make one from.
    pub reset: bool,
}

pub struct Reproject {
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    params: wgpu::Buffer,
    /// The last frame's `Inputs::color`.
    previous: Option<wgpu::Texture>,
    output: wgpu::TextureView,
}

impl Reproject {
    /// For frames of `display` size.
    pub fn new(device: &wgpu::Device, display: [u32; 2]) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Frame generation reprojection"),
            source: wgpu::ShaderSource::Wgsl(include_str!("reproject.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Frame generation reprojection"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_full"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_made"),
                compilation_options: Default::default(),
                targets: &[Some(OUTPUT.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let output = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("Frame generation reprojected frame"),
                size: wgpu::Extent3d {
                    width: display[0].max(1),
                    height: display[1].max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: OUTPUT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            })
            .create_view(&Default::default());
        Self {
            pipeline,
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Frame generation reprojection"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            params: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Frame generation reprojection"),
                size: std::mem::size_of::<Params>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            previous: None,
            output,
        }
    }

    /// The made frame, into `output`, unless there is no previous frame (a reset, the
    /// first frame or a new size); keeps this frame's colour for the next.
    pub fn dispatch(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        inputs: &Inputs,
    ) {
        let color = inputs.color.texture();
        let mut reset = inputs.reset;
        if self
            .previous
            .as_ref()
            .is_none_or(|p| p.size() != color.size() || p.format() != color.format())
        {
            self.previous = Some(device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Frame generation previous frame"),
                size: color.size(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: color.format(),
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            }));
            reset = true;
        }
        let previous = self.previous.as_ref().expect("made above");
        if !reset {
            let params = Params {
                reproject: inputs.reproject.to_cols_array_2d(),
                render_size: inputs.render_size.map(|v| v.max(1) as f32),
                jitter: inputs.jitter,
            };
            queue.write_buffer(&self.params, 0, bytemuck::bytes_of(&params));
            let previous_view = previous.create_view(&Default::default());
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Frame generation reprojection"),
                layout: &self.pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.params.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(inputs.depth),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(inputs.color),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&previous_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(inputs.object_motion),
                    },
                ],
            });
            let attachments = [Some(wgpu::RenderPassColorAttachment {
                view: &self.output,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })];
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Frame generation reprojection"),
                color_attachments: &attachments,
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_texture(
            color.as_image_copy(),
            previous.as_image_copy(),
            color.size(),
        );
    }

    /// The made frame: the screen's size, `Rgba16Float`, sampled; its colour is encoded as
    /// `Inputs::color`'s is.
    pub fn output(&self) -> &wgpu::TextureView {
        &self.output
    }
}

#[cfg(test)]
mod tests {
    use super::super::optical_flow::tests as of;
    use super::*;

    #[test]
    fn shader_validates() {
        let module = naga::front::wgsl::parse_str(include_str!("reproject.wgsl")).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
    }

    fn texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: [u32; 2],
        format: wgpu::TextureFormat,
        texel: &[u8],
    ) -> wgpu::TextureView {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let pixels: Vec<u8> = texel
            .iter()
            .copied()
            .cycle()
            .take(texel.len() * (size[0] * size[1]) as usize)
            .collect();
        queue.write_texture(
            texture.as_image_copy(),
            &pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size[0] * texel.len() as u32),
                rows_per_image: None,
            },
            texture.size(),
        );
        texture.create_view(&Default::default())
    }

    fn f16(bits: u16) -> f32 {
        let (sign, exp, frac) = (bits >> 15, (bits >> 10) & 0x1f, bits & 0x3ff);
        let magnitude = match exp {
            0 => f32::from(frac) * 2f32.powi(-24),
            _ => (1.0 + f32::from(frac) / 1024.0) * 2f32.powi(i32::from(exp) - 15),
        };
        if sign == 1 { -magnitude } else { magnitude }
    }

    /// The camera stood still and nothing moved on its own: the made frame is the last
    /// frame and this one half and half, and the first frame makes none.
    #[test]
    fn a_still_camera_blends_the_two_frames() {
        let _gpu = crate::gfx::test_gpu::lock();
        let Some((device, queue)) = of::device() else {
            return;
        };
        let size = [8, 8];
        let rgba = wgpu::TextureFormat::Rgba8Unorm;
        let red = texture(&device, &queue, size, rgba, &[200, 0, 0, 255]);
        let blue = texture(&device, &queue, size, rgba, &[0, 0, 200, 255]);
        let depth = texture(
            &device,
            &queue,
            size,
            wgpu::TextureFormat::R32Float,
            &0.5f32.to_le_bytes(),
        );
        let motion = texture(&device, &queue, size, super::super::motion::FORMAT, &[0; 8]);
        let mut fast = Reproject::new(&device, size);
        let inputs = |color, reset| Inputs {
            color,
            depth: &depth,
            object_motion: &motion,
            render_size: size,
            jitter: [0.0; 2],
            reproject: Mat4::IDENTITY,
            reset,
        };
        for (color, reset) in [(&blue, true), (&red, false)] {
            let mut encoder = device.create_command_encoder(&Default::default());
            fast.dispatch(&device, &queue, &mut encoder, &inputs(color, reset));
            queue.submit([encoder.finish()]);
        }
        let texels = of::read(&device, &queue, fast.output().texture(), 8);
        for texel in texels.chunks(8) {
            let channel = |i: usize| f16(u16::from_le_bytes([texel[i * 2], texel[i * 2 + 1]]));
            let half = 100.0 / 255.0;
            assert!((channel(0) - half).abs() < 0.01, "red {}", channel(0));
            assert!(channel(1).abs() < 0.01, "green {}", channel(1));
            assert!((channel(2) - half).abs() < 0.01, "blue {}", channel(2));
        }
    }
}
