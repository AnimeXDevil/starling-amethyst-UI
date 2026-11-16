use super::{HudAtlas, PackedImage, Result, SMALLEST_SHRUNK_SIDE, TextureFlags, halved};
use std::collections::HashMap;

impl HudAtlas {
    /// Fit static sprites to a small adapter. Menu button backgrounds give up one level of
    /// detail before world sprites. Glyphs and dynamic map/skin/GUI slots keep their size.
    pub(super) fn fit_mobile_atlas(
        images: &mut [PackedImage],
        gui_sizes: &mut HashMap<String, (u32, u32)>,
        flags: &HashMap<String, TextureFlags>,
        limit: u32,
    ) -> Result<()> {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        enum Priority {
            ButtonBackground,
            World,
            Interface,
        }

        let shrink_rank = |(name, w, h, rgba): &PackedImage,
                           original_sizes: &HashMap<String, (u32, u32)>| {
            let path = name.split_once(':').map_or(name.as_str(), |(_, path)| path);
            let side = (*w).max(*h);
            // The two standard skin layouts use at most 64×64 texels. The slots for
            // runtime patches keep their own pixel coordinates.
            if path.contains("font/")
                || name.starts_with("horizon/")
                || (path.starts_with("entity/player/") && side <= 64)
                || side <= SMALLEST_SHRUNK_SIDE
                || rgba.len() < (*w as usize * *h as usize * 4)
            {
                return None;
            }
            let button_background = matches!(
                path,
                "gui/sprites/widget/button"
                    | "gui/sprites/widget/button_disabled"
                    | "gui/sprites/widget/button_highlighted"
                    | "gui/sprites/widget/realms_button"
                    | "gui/sprites/widget/realms_button_disabled"
                    | "gui/sprites/widget/realms_button_highlighted"
                    | "gui/bedrock/controls/large_button"
                    | "gui/bedrock/controls/large_button_pressed"
            );
            let priority = if button_background && !original_sizes.contains_key(name) {
                Priority::ButtonBackground
            } else if path.starts_with("gui/") || path.starts_with("ui/") {
                Priority::Interface
            } else {
                Priority::World
            };
            Some((priority, std::cmp::Reverse(side)))
        };
        let pack = |dimensions: &[PackedImage]| {
            let mut sorted = dimensions.to_vec();
            sorted.sort_by(|a, b| b.2.cmp(&a.2).then(b.1.cmp(&a.1)));
            if limit <= 4096 {
                Self::mobile_layout(&sorted, limit)
            } else {
                Self::layout(&sorted, limit)
            }
        };
        let borders: Vec<_> = images
            .iter()
            .map(|(name, ..)| {
                if flags.get(name).is_some_and(|f| f.blur) {
                    2
                } else {
                    0
                }
            })
            .collect();
        let mut dimensions: Vec<PackedImage> = images
            .iter()
            .zip(&borders)
            .map(|((_, w, h, _), border)| (String::new(), w + border, h + border, Vec::new()))
            .collect();
        let Err(mut error) = pack(&dimensions) else {
            return Ok(());
        };
        loop {
            // Repack after reducing one size group. Small sprites can keep all their
            // texels when making room in a few large sprites is enough.
            let Some(rank @ (priority, std::cmp::Reverse(side))) = images
                .iter()
                .filter_map(|image| shrink_rank(image, gui_sizes))
                .min()
            else {
                return Err(error);
            };
            let mut group: Vec<_> = images
                .iter()
                .enumerate()
                .filter_map(|(index, image)| {
                    (shrink_rank(image, gui_sizes) == Some(rank)).then_some(index)
                })
                .collect();
            group.sort_by_key(|&index| {
                let (_, w, h, _) = &images[index];
                std::cmp::Reverse(u64::from(*w) * u64::from(*h))
            });
            let set_trial = |dimensions: &mut [PackedImage], count| {
                for (index, &sprite) in group.iter().enumerate() {
                    let (_, w, h, _) = &images[sprite];
                    let divisor = if index < count { 2 } else { 1 };
                    dimensions[sprite].1 = w.div_ceil(divisor) + borders[sprite];
                    dimensions[sprite].2 = h.div_ceil(divisor) + borders[sprite];
                }
            };
            let mut count = group.len();
            set_trial(&mut dimensions, count);
            let remaining = pack(&dimensions).err();
            if remaining.is_none() {
                // Keep a known fit while trying smaller prefixes of this size group.
                let mut failed = 0;
                while count - failed > 1 {
                    let middle = failed + (count - failed) / 2;
                    set_trial(&mut dimensions, middle);
                    if pack(&dimensions).is_ok() {
                        count = middle;
                    } else {
                        failed = middle;
                    }
                }
            }
            for &index in &group[..count] {
                let image = &mut images[index];
                gui_sizes
                    .entry(image.0.clone())
                    .or_insert((image.1, image.2));
                *image = halved(image, priority == Priority::ButtonBackground);
            }
            let category = match priority {
                Priority::ButtonBackground => "button background",
                Priority::World => "non-UI",
                Priority::Interface => "interface",
            };
            log::info!(
                "Reduced {count} {category} atlas sprites with a {side}px side to fit the adapter's {limit}px texture limit"
            );
            let Some(next_error) = remaining else {
                return Ok(());
            };
            error = next_error;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_atlas_reduces_button_backgrounds_before_world_sprites() {
        let mut button = vec![255; 400 * 20 * 4];
        for y in 0..20 {
            for x in 0..400 {
                if x == 0 || x == 399 || y == 0 || y == 19 {
                    let start = (y * 400 + x) * 4;
                    button[start..start + 4].copy_from_slice(&[0, 0, 0, 255]);
                }
            }
        }
        let mut images = [
            (
                "minecraft:gui/sprites/widget/button".into(),
                400,
                20,
                button,
            ),
            ("entity/mob".into(), 400, 240, vec![255; 400 * 240 * 4]),
        ];
        let mut full_sizes = HashMap::new();
        HudAtlas::fit_mobile_atlas(&mut images, &mut full_sizes, &HashMap::new(), 768).unwrap();

        assert_eq!((images[0].1, images[0].2), (200, 10));
        assert_eq!((images[1].1, images[1].2), (400, 240));
        assert_eq!(
            full_sizes,
            HashMap::from([(images[0].0.clone(), (400, 20))])
        );
        for (index, pixel) in images[0].3.as_chunks::<4>().0.iter().enumerate() {
            let (x, y) = (index % 200, index / 200);
            let expected = if x == 0 || x == 199 || y == 0 || y == 9 {
                [0, 0, 0, 255]
            } else {
                [255; 4]
            };
            assert_eq!(*pixel, expected, "button pixel ({x}, {y})");
        }
    }

    #[test]
    fn small_atlas_reduces_only_the_sprites_needed_from_the_last_size_group() {
        let mut images: Vec<PackedImage> = (0..4)
            .map(|i| (format!("entity/{i}"), 200, 200, vec![255; 200 * 200 * 4]))
            .collect();
        let mut full_sizes = HashMap::new();
        HudAtlas::fit_mobile_atlas(&mut images, &mut full_sizes, &HashMap::new(), 768).unwrap();

        assert_eq!(
            images
                .iter()
                .filter(|image| image.1 == 200 && image.2 == 200)
                .count(),
            3
        );
        let reduced = images
            .iter()
            .find(|image| image.1 == 100 && image.2 == 100)
            .unwrap();
        assert_eq!(full_sizes, HashMap::from([(reduced.0.clone(), (200, 200))]));
    }
}
