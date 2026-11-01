//! Albedo textures for the viewport.
//!
//! The scene names its images as `ExtResource`s; `gamecraft_gltf::load_textures`
//! reads them from the project folder (the same loader the glTF export uses) and
//! this cache uploads each one to egui once, with repeat wrapping because the
//! triplanar UVs run outside 0..1.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gamecraft_godot::SceneFile;

/// `(id, path)` of every external resource in the scene.
type ExtKey = Vec<(String, String)>;

/// Decoded-on-demand textures for the loaded scene.
#[derive(Default)]
pub struct TexCache {
    key: Option<(Option<PathBuf>, ExtKey)>,
    bytes: gamecraft_gltf::Textures,
    handles: HashMap<String, Option<egui::TextureHandle>>,
    /// CPU copies with mip chains for the software rasterizer.
    mips: HashMap<String, Option<Arc<crate::raster::Mips>>>,
    /// The rasterizer's output, updated in place each frame.
    frame: Option<egui::TextureHandle>,
}

impl std::fmt::Debug for TexCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TexCache")
            .field("images", &self.bytes.len())
            .field("uploaded", &self.handles.len())
            .finish_non_exhaustive()
    }
}

impl TexCache {
    /// Reload the image bytes when the scene file or its external resources changed.
    pub fn sync(&mut self, scene: Option<&Path>, source: Option<&SceneFile>) {
        let key = (
            scene.map(Path::to_path_buf),
            source
                .map(|file| {
                    file.ext_resources
                        .iter()
                        .map(|ext| (ext.id.clone(), ext.path.clone()))
                        .collect()
                })
                .unwrap_or_default(),
        );
        if self.key.as_ref() == Some(&key) {
            return;
        }
        self.bytes = match (source, scene) {
            (Some(file), Some(path)) => {
                gamecraft_gltf::load_textures(file, gamecraft_gltf::project_root(path).as_path())
            }
            _ => gamecraft_gltf::Textures::new(),
        };
        self.handles.clear();
        self.mips.clear();
        self.key = Some(key);
    }

    /// The decoded image with its mip chain, for per-pixel shading. `None` when the
    /// image is missing or cannot be decoded.
    pub fn mips(&mut self, texture: &str) -> Option<Arc<crate::raster::Mips>> {
        if !self.mips.contains_key(texture) {
            let decoded = self.bytes.get(texture).and_then(|bytes| {
                let rgba = image::load_from_memory(bytes).ok()?.to_rgba8();
                Some(Arc::new(crate::raster::Mips::new(rgba)))
            });
            self.mips.insert(texture.to_string(), decoded);
        }
        self.mips.get(texture).and_then(Clone::clone)
    }

    /// Upload the rasterizer frame, reusing one texture across frames.
    pub fn frame_texture(
        &mut self,
        ctx: &egui::Context,
        image: egui::ColorImage,
    ) -> egui::TextureId {
        if let Some(handle) = &mut self.frame {
            handle.set(image, egui::TextureOptions::LINEAR);
            return handle.id();
        }
        let handle = ctx.load_texture(
            "gamecraft-raster-frame",
            image,
            egui::TextureOptions::LINEAR,
        );
        let id = handle.id();
        self.frame = Some(handle);
        id
    }

    /// The egui texture for an `ExtResource` id, uploading it on first use.
    /// `None` when the image is missing or cannot be decoded: the triangle then
    /// falls back to its flat colour.
    pub fn id(&mut self, ctx: &egui::Context, texture: &str) -> Option<egui::TextureId> {
        if !self.handles.contains_key(texture) {
            let handle = self.bytes.get(texture).and_then(|bytes| {
                let rgba = image::load_from_memory(bytes).ok()?.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                Some(ctx.load_texture(
                    format!("gamecraft-{texture}"),
                    color,
                    egui::TextureOptions {
                        wrap_mode: egui::TextureWrapMode::Repeat,
                        mipmap_mode: Some(egui::TextureFilter::Linear),
                        ..egui::TextureOptions::LINEAR
                    },
                ))
            });
            self.handles.insert(texture.to_string(), handle);
        }
        self.handles
            .get(texture)
            .and_then(Option::as_ref)
            .map(egui::TextureHandle::id)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn project() -> (PathBuf, SceneFile) {
        let dir = std::env::temp_dir().join(format!("gc-tex-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("t")).expect("dir");
        std::fs::write(dir.join("project.godot"), "config_version=5\n").expect("project");
        image::RgbaImage::from_pixel(4, 4, image::Rgba([200, 30, 30, 255]))
            .save(dir.join("t").join("a.png"))
            .expect("png");
        let text = "[gd_scene format=3]\n\n[ext_resource type=\"Texture2D\" path=\"res://t/a.png\" id=\"1_a\"]\n[ext_resource type=\"Texture2D\" path=\"res://t/gone.png\" id=\"2_b\"]\n\n[node name=\"W\" type=\"Node3D\"]\n";
        (
            dir.clone(),
            gamecraft_godot::parse_tscn(text).expect("parse"),
        )
    }

    #[test]
    fn uploads_present_images_and_skips_missing_ones() {
        let (dir, file) = project();
        let scene = dir.join("scene.tscn");
        let ctx = egui::Context::default();
        let mut cache = TexCache::default();
        cache.sync(Some(&scene), Some(&file));
        assert!(cache.id(&ctx, "1_a").is_some());
        assert!(
            cache.id(&ctx, "2_b").is_none(),
            "missing file must not upload"
        );
        assert!(cache.id(&ctx, "nope").is_none());
        // same key: nothing reloads, handles survive
        cache.sync(Some(&scene), Some(&file));
        assert_eq!(cache.handles.len(), 3);
        // no scene path: textures cannot be located
        cache.sync(None, Some(&file));
        assert!(cache.id(&ctx, "1_a").is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
