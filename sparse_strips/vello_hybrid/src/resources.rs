// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Persistent renderer resources shared across frames.

use crate::render::common::IMAGE_PADDING;
#[cfg(feature = "text")]
use crate::text::GlyphAtlasResources;
#[cfg(feature = "text")]
use glifo::GlyphPrepCache;
use vello_common::image_cache::ImageCache;
use vello_common::multi_atlas::AtlasConfig;

/// Persistent resources required by Vello Hybrid for rendering.
///
/// You should create one such instance per renderer.
#[derive(Debug)]
pub struct Resources {
    pub(crate) image_cache: ImageCache,
    #[cfg(feature = "text")]
    pub(crate) glyph_prep_cache: GlyphPrepCache,
    #[cfg(feature = "text")]
    pub(crate) glyph_resources: Option<GlyphAtlasResources>,
}

impl Resources {
    /// Create a new set of renderer resources.
    pub fn new() -> Self {
        Self::new_with_config(AtlasConfig::default())
    }

    /// Create a new set of renderer resources with a custom image/glyph atlas configuration.
    ///
    /// This should match the
    /// [`image_atlas_config`](crate::MemorySettings::image_atlas_config) passed to the renderer so
    /// that allocations and the GPU atlas texture agree on the atlas size.
    // TODO: Requiring callers to keep this config in sync with the renderer's
    // `memory_settings.image_atlas_config` by hand is a footgun; tie them together so they can't
    // diverge. Note the renderer also normalizes the config against the device limits, so even
    // matching configs here can still end up disagreeing.
    pub fn new_with_config(image_atlas_config: AtlasConfig) -> Self {
        Self {
            image_cache: ImageCache::new_with_config(image_atlas_config),
            #[cfg(feature = "text")]
            glyph_prep_cache: GlyphPrepCache::default(),
            // Will be initialized lazily.
            #[cfg(feature = "text")]
            glyph_resources: None,
        }
    }

    /// The width and height of the largest image that [`Renderer::upload_image`] can place in
    /// the image atlas, including the padding the renderer adds around uploaded images.
    ///
    /// Images larger than this in either dimension fail to upload with
    /// [`AtlasError::TextureTooLarge`](vello_common::multi_atlas::AtlasError::TextureTooLarge)
    /// and should be downscaled by the caller first.
    ///
    /// This follows the config given to [`Resources::new_with_config`], so it can be larger than
    /// what the device supports if that config was not normalized to the device limits.
    ///
    /// [`Renderer::upload_image`]: crate::Renderer::upload_image
    pub fn max_image_size(&self) -> (u32, u32) {
        let (width, height) = self.image_cache.atlas_manager().config().atlas_size;
        let padding = u32::from(IMAGE_PADDING) * 2;
        (
            width.saturating_sub(padding),
            height.saturating_sub(padding),
        )
    }
}

impl Default for Resources {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_image_size_follows_the_atlas_config() {
        let resources = Resources::new_with_config(AtlasConfig {
            atlas_size: (512, 256),
            ..AtlasConfig::default()
        });
        let padding = u32::from(IMAGE_PADDING) * 2;
        assert_eq!(resources.max_image_size(), (512 - padding, 256 - padding));
    }
}
