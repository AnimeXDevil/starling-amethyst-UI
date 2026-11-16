use super::{HudAtlas, PackedImage, Result, SMALLEST_SHRUNK_SIDE, TextureFlags, halved};
use std::collections::HashMap;

impl HudAtlas {
    /// Fit static sprites to a small adapter without shrinking pixel icons, glyphs or dynamic
    /// map/skin/GUI slots. Terrain uses separate texture arrays and is unchanged.
    pub(super) fn fit_mobile_atlas(
        images: &mut [PackedImage],
        gui_sizes: &mut HashMap<String, (u32, u32)>,
        flags: &HashMap<String, TextureFlags>,
        limit: u32,
    ) -> Result<()> {
        let shrink_rank = |(name, w, h, rgba): &PackedImage| {
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
            let interface = path.starts_with("gui/") || path.starts_with("ui/");
            Some((interface, std::cmp::Reverse(side)))
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
            let Some(rank @ (interface, std::cmp::Reverse(side))) =
                images.iter().filter_map(shrink_rank).min()
            else {
                return Err(error);
            };
            let mut group: Vec<_> = images
                .iter()
                .enumerate()
                .filter_map(|(index, image)| (shrink_rank(image) == Some(rank)).then_some(index))
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
                *image = halved(image);
            }
            log::info!(
                "Reduced {count} {} atlas sprites with a {side}px side to fit the adapter's {limit}px texture limit",
                if interface { "interface" } else { "non-UI" }
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
