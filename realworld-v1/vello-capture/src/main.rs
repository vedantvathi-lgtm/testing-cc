//! Capture vello's real GPU work for its own test scenes, for replay on the wgpu fork under test.
//!
//! For every scene in `scenes::test_scenes::test_scenes()` (vello a9f11bb), set up exactly like vello's
//! `research/examples/headless` (time 0, complexity 0, 1000x1000 unless the scene sets a
//! resolution, base colour from the scene or black, AaConfig::Area) and write:
//!   <out>/<i>-<name>.json   the Recording produced by vello's own `low_level::Render`
//!                           (`render_encoding_coarse(robust = false)` + `record_fine`, the same
//!                           calls `Renderer::render_to_texture` makes), with byte payloads in
//!   <out>/<i>-<name>.blob
//!   <out>/<i>-<name>.ref.<k>.rgba  every distinct image vello's own `Renderer` produced for that
//!                           scene in VELLO_REFS (default 40) renders on wgpu 30, counts in .refs.json
//! and once:
//!   <out>/shaders.json      the shader table: vello_shaders' preprocessed WGSL, the bind layout
//!                           vello's engine uses (`full_shaders`, copied verbatim), workgroup size.

use anyhow::{bail, Result};
use scenes::{ImageCache, SceneParams, SimpleText};
use serde_json::{json, Value};
use std::io::Write;
use vello::kurbo::{Affine, Vec2};
use vello::low_level::{BindType, Command, FullShaders, ImageFormat, ImageProxy, Recording, Render, ResourceProxy, ShaderId};
use vello::peniko::color::palette;
use vello::wgpu;
use vello::{AaConfig, AaSupport, RenderParams, Renderer, RendererOptions, Scene};

const VELLO_REV: &str = "a9f11bbe752c391a2e420657d65cbd1fc97f4fae";

struct Entry {
    field: &'static str,
    source: vello_shaders::ComputeShader<'static>,
    layout: Vec<BindType>,
}

fn table() -> Vec<Entry> {
    use BindType::*;
    let s = vello_shaders::SHADERS;
    let fine = vec![Uniform, BufReadOnly, BufReadOnly, BufReadOnly, Buffer, Image(ImageFormat::Rgba8), ImageRead(ImageFormat::Rgba8), ImageRead(ImageFormat::Rgba8)];
    // Order and layouts copied from vello research/vello_research/src/shaders.rs `full_shaders`
    // (area-only AA support, as the headless example configures).
    vec![
        Entry { field: "pathtag_reduce", source: s.pathtag_reduce, layout: vec![Uniform, BufReadOnly, Buffer] },
        Entry { field: "pathtag_reduce2", source: s.pathtag_reduce2, layout: vec![BufReadOnly, Buffer] },
        Entry { field: "pathtag_scan1", source: s.pathtag_scan1, layout: vec![BufReadOnly, BufReadOnly, Buffer] },
        Entry { field: "pathtag_scan", source: s.pathtag_scan_small, layout: vec![Uniform, BufReadOnly, BufReadOnly, Buffer] },
        Entry { field: "pathtag_scan_large", source: s.pathtag_scan_large, layout: vec![Uniform, BufReadOnly, BufReadOnly, Buffer] },
        Entry { field: "bbox_clear", source: s.bbox_clear, layout: vec![Uniform, Buffer] },
        Entry { field: "flatten", source: s.flatten, layout: vec![Uniform, BufReadOnly, BufReadOnly, Buffer, Buffer, Buffer] },
        Entry { field: "draw_reduce", source: s.draw_reduce, layout: vec![Uniform, BufReadOnly, Buffer] },
        Entry { field: "draw_leaf", source: s.draw_leaf, layout: vec![Uniform, BufReadOnly, BufReadOnly, BufReadOnly, Buffer, Buffer, Buffer] },
        Entry { field: "clip_reduce", source: s.clip_reduce, layout: vec![BufReadOnly, BufReadOnly, Buffer, Buffer] },
        Entry { field: "clip_leaf", source: s.clip_leaf, layout: vec![Uniform, BufReadOnly, BufReadOnly, BufReadOnly, BufReadOnly, Buffer, Buffer] },
        Entry { field: "binning", source: s.binning, layout: vec![Uniform, BufReadOnly, BufReadOnly, BufReadOnly, Buffer, Buffer, Buffer, Buffer] },
        Entry { field: "tile_alloc", source: s.tile_alloc, layout: vec![Uniform, BufReadOnly, BufReadOnly, Buffer, Buffer, Buffer] },
        Entry { field: "path_count_setup", source: s.path_count_setup, layout: vec![Buffer, Buffer] },
        Entry { field: "path_count", source: s.path_count, layout: vec![Uniform, Buffer, BufReadOnly, BufReadOnly, Buffer, Buffer] },
        Entry { field: "backdrop", source: s.backdrop_dyn, layout: vec![Uniform, Buffer, BufReadOnly, Buffer] },
        Entry { field: "coarse", source: s.coarse, layout: vec![Uniform, BufReadOnly, BufReadOnly, BufReadOnly, BufReadOnly, BufReadOnly, Buffer, Buffer, Buffer] },
        Entry { field: "path_tiling_setup", source: s.path_tiling_setup, layout: vec![Buffer, Buffer, Buffer] },
        Entry { field: "path_tiling", source: s.path_tiling, layout: vec![Buffer, BufReadOnly, BufReadOnly, BufReadOnly, BufReadOnly, Buffer] },
        Entry { field: "fine_area", source: s.fine_area, layout: fine },
    ]
}

fn bind_name(b: &BindType) -> &'static str {
    match b {
        BindType::Buffer => "buffer",
        BindType::BufReadOnly => "buffer_read_only",
        BindType::Uniform => "uniform",
        BindType::Image(ImageFormat::Rgba8) => "image_rgba8",
        BindType::Image(ImageFormat::Bgra8) => "image_bgra8",
        BindType::ImageRead(ImageFormat::Rgba8) => "image_read_rgba8",
        BindType::ImageRead(ImageFormat::Bgra8) => "image_read_bgra8",
    }
}

fn reflected_name(b: &vello_shaders::BindType) -> &'static str {
    use vello_shaders::BindType as R;
    match b {
        R::Buffer => "buffer",
        R::BufReadOnly => "buffer_read_only",
        R::Uniform => "uniform",
        R::Image => "image",
        R::ImageRead => "image_read",
    }
}

fn full_shaders(entries: &[Entry]) -> FullShaders {
    let id = |f: &str| ShaderId(entries.iter().position(|e| e.field == f).unwrap());
    FullShaders {
        pathtag_reduce: id("pathtag_reduce"),
        pathtag_reduce2: id("pathtag_reduce2"),
        pathtag_scan1: id("pathtag_scan1"),
        pathtag_scan: id("pathtag_scan"),
        pathtag_scan_large: id("pathtag_scan_large"),
        bbox_clear: id("bbox_clear"),
        flatten: id("flatten"),
        draw_reduce: id("draw_reduce"),
        draw_leaf: id("draw_leaf"),
        clip_reduce: id("clip_reduce"),
        clip_leaf: id("clip_leaf"),
        binning: id("binning"),
        tile_alloc: id("tile_alloc"),
        path_count_setup: id("path_count_setup"),
        path_count: id("path_count"),
        backdrop: id("backdrop"),
        coarse: id("coarse"),
        path_tiling_setup: id("path_tiling_setup"),
        path_tiling: id("path_tiling"),
        fine_area: Some(id("fine_area")),
        fine_msaa8: None,
        fine_msaa16: None,
        pathtag_is_cpu: false,
    }
}

fn format_name(f: ImageFormat) -> &'static str {
    match f {
        ImageFormat::Rgba8 => "rgba8",
        ImageFormat::Bgra8 => "bgra8",
    }
}

fn image_json(p: &ImageProxy) -> Value {
    json!({"id": p.id.0.get(), "width": p.width, "height": p.height, "format": format_name(p.format)})
}

fn resource_json(r: &ResourceProxy) -> Value {
    match r {
        ResourceProxy::Buffer(b) => json!({"kind": "buffer", "id": b.id.0.get(), "size": b.size, "name": b.name}),
        ResourceProxy::BufferRange { proxy, offset, size } => {
            json!({"kind": "range", "id": proxy.id.0.get(), "size": proxy.size, "name": proxy.name, "offset": offset, "range_size": size})
        }
        ResourceProxy::Image(i) => json!({"kind": "image", "image": image_json(i)}),
    }
}

fn serialize(recording: &Recording, blob: &mut Vec<u8>) -> Vec<Value> {
    let mut put = |bytes: &[u8]| {
        let off = blob.len();
        blob.extend_from_slice(bytes);
        json!([off, bytes.len()])
    };
    recording
        .commands
        .iter()
        .map(|c| match c {
            Command::Upload(b, bytes) => json!({"op": "upload", "buf": {"id": b.id.0.get(), "size": b.size, "name": b.name}, "data": put(bytes)}),
            Command::UploadUniform(b, bytes) => json!({"op": "upload_uniform", "buf": {"id": b.id.0.get(), "size": b.size, "name": b.name}, "data": put(bytes)}),
            Command::UploadImage(i, bytes) => json!({"op": "upload_image", "image": image_json(i), "data": put(bytes)}),
            Command::WriteImage(i, [x, y], image) => json!({
                "op": "write_image", "image": image_json(i), "x": x, "y": y,
                "width": image.width, "height": image.height, "data": put(image.data.data()),
            }),
            Command::Download(b) => json!({"op": "download", "buf": {"id": b.id.0.get(), "size": b.size, "name": b.name}}),
            Command::Clear(b, offset, size) => json!({"op": "clear", "buf": {"id": b.id.0.get(), "size": b.size, "name": b.name}, "offset": offset, "len": size}),
            Command::FreeBuffer(b) => json!({"op": "free_buffer", "id": b.id.0.get()}),
            Command::FreeImage(i) => json!({"op": "free_image", "id": i.id.0.get()}),
            Command::Dispatch(s, (x, y, z), res) => json!({"op": "dispatch", "shader": s.0, "wg": [x, y, z], "bindings": res.iter().map(resource_json).collect::<Vec<_>>()}),
            Command::DispatchIndirect(s, b, offset, res) => json!({
                "op": "dispatch_indirect", "shader": s.0, "buf": {"id": b.id.0.get(), "size": b.size, "name": b.name}, "offset": offset,
                "bindings": res.iter().map(resource_json).collect::<Vec<_>>(),
            }),
        })
        .collect()
}

fn render_reference(device: &wgpu::Device, queue: &wgpu::Queue, renderer: &mut Renderer, scene: &Scene, params: &RenderParams) -> Result<Vec<u8>> {
    let (width, height) = (params.width, params.height);
    let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("target"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    if renderer.render_to_texture(device, queue, scene, &view, params).is_err() {
        bail!("vello render_to_texture failed");
    }
    let padded = (width * 4).next_multiple_of(256);
    let buf = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: u64::from(padded * height), usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let mut enc = device.create_command_encoder(&Default::default());
    enc.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: None } },
        size,
    );
    queue.submit([enc.finish()]);
    let slice = buf.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| ());
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let data = slice.get_mapped_range().map_err(|e| anyhow::anyhow!("map: {e:?}"))?;
    let mut out = Vec::with_capacity((width * height * 4) as usize);
    for row in 0..height {
        let start = (row * padded) as usize;
        out.extend_from_slice(&data[start..start + (width * 4) as usize]);
    }
    Ok(out)
}

fn main() -> Result<()> {
    let out = std::path::PathBuf::from(std::env::args().nth(1).expect("usage: bce-vello-capture <out-dir> [scene-index...]"));
    let only: Vec<usize> = std::env::args().skip(2).filter_map(|a| a.parse().ok()).collect();
    std::fs::create_dir_all(&out)?;

    let entries = table();
    let mut shader_json = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        let reflected: Vec<&str> = e.source.bindings.iter().map(reflected_name).collect();
        let layout: Vec<&str> = e.layout.iter().map(bind_name).collect();
        // Cross-check the copied layout against vello_shaders' reflection (images: kind only).
        assert_eq!(reflected.len(), layout.len(), "{}: layout length", e.field);
        for (r, l) in reflected.iter().zip(&layout) {
            assert!(l.starts_with(r), "{}: reflected {r} vs layout {l}", e.field);
        }
        shader_json.push(json!({
            "id": i, "field": e.field, "name": e.source.name, "workgroup_size": e.source.workgroup_size,
            "layout": layout, "wgsl_binding_indices": e.source.wgsl.binding_indices, "wgsl": e.source.wgsl.code,
        }));
    }
    std::fs::write(out.join("shaders.json"), serde_json::to_vec_pretty(&json!({"vello_rev": VELLO_REV, "shaders": shader_json}))?)?;

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor { backends: wgpu::Backends::VULKAN, ..wgpu::InstanceDescriptor::new_without_display_handle() });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions { power_preference: wgpu::PowerPreference::HighPerformance, ..Default::default() }))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let mut renderer = Renderer::new(&device, RendererOptions { use_cpu: false, antialiasing_support: AaSupport::area_only(), num_init_threads: std::num::NonZeroUsize::new(1), pipeline_cache: None })
        .map_err(|e| anyhow::anyhow!("renderer: {e}"))?;
    let shaders = full_shaders(&entries);

    let mut index = Vec::new();
    let mut set = scenes::test_scenes::test_scenes();
    for (i, example) in set.scenes.iter_mut().enumerate() {
        if !only.is_empty() && !only.contains(&i) {
            continue;
        }
        let name = example.config.name.clone();
        let safe: String = name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        let stem = format!("{i:02}-{safe}");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<Value> {
            let mut fragment = Scene::new();
            let mut text = SimpleText::new();
            let mut images = ImageCache::new();
            let mut params = SceneParams { time: 0.0, text: &mut text, images: &mut images, resolution: None, base_color: None, interactive: false, complexity: 0 };
            example.function.render(&mut fragment, &mut params);
            let mut transform = Affine::IDENTITY;
            let (width, height) = if let Some(resolution) = params.resolution {
                let (w, h) = (resolution.x.ceil() as u32, resolution.y.ceil() as u32);
                let factor = Vec2::new(w as f64, h as f64);
                transform *= Affine::scale((factor.x / resolution.x).min(factor.y / resolution.y));
                (w, h)
            } else {
                (1000, 1000)
            };
            let base_color = params.base_color.unwrap_or(palette::css::BLACK);
            let render_params = RenderParams { base_color, width, height, antialiasing_method: AaConfig::Area };
            let mut scene = Scene::new();
            scene.append(&fragment, Some(transform));

            // vello's own renderer is not bit-deterministic on every scene (e.g. blend_grid flips one
            // pixel in ~1 of 4 renders), so keep every distinct image it produces in N renders.
            let n_refs: usize = std::env::var("VELLO_REFS").ok().and_then(|v| v.parse().ok()).unwrap_or(40);
            let mut states: Vec<(Vec<u8>, usize)> = Vec::new();
            for _ in 0..n_refs {
                let img = render_reference(&device, &queue, &mut renderer, &scene, &render_params)?;
                match states.iter_mut().find(|(s, _)| *s == img) {
                    Some((_, count)) => *count += 1,
                    None => states.push((img, 1)),
                }
            }
            let mut ref_meta = Vec::new();
            for (k, (img, count)) in states.iter().enumerate() {
                std::fs::write(out.join(format!("{stem}.ref.{k}.rgba")), img)?;
                ref_meta.push(json!({"file": format!("{stem}.ref.{k}.rgba"), "count": count}));
            }
            std::fs::write(out.join(format!("{stem}.refs.json")), serde_json::to_vec_pretty(&json!({"renders": n_refs, "states": ref_meta}))?)?;
            let mut resolver = vello::low_level::vello_encoding::Resolver::new();
            let mut atlas: Option<ImageProxy> = None;
            let mut render = Render::new();
            let mut recording = render.render_encoding_coarse(scene.encoding(), &mut resolver, &shaders, &mut atlas, &render_params, false);
            let out_image = render.out_image();
            render.record_fine(&shaders, &mut recording);
            let mut blob = Vec::new();
            let commands = serialize(&recording, &mut blob);
            let n = commands.len();
            let c = base_color.to_rgba8();
            let rec = json!({
                "vello_rev": VELLO_REV, "scene_index": i, "scene": name, "width": width, "height": height,
                "base_color_rgba8": [c.r, c.g, c.b, c.a], "aa": "area", "out_image": image_json(&out_image), "commands": commands,
            });
            std::fs::write(out.join(format!("{stem}.json")), serde_json::to_vec(&rec)?)?;
            std::fs::File::create(out.join(format!("{stem}.blob")))?.write_all(&blob)?;
            Ok(json!({"index": i, "scene": name, "stem": stem, "width": width, "height": height, "commands": n, "blob_bytes": blob.len(), "vello_states": states.len(), "vello_renders": n_refs}))
        }));
        let entry = match result {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => json!({"index": i, "scene": name, "error": e.to_string()}),
            Err(_) => json!({"index": i, "scene": name, "error": "panic"}),
        };
        println!("{entry}");
        index.push(entry);
    }
    std::fs::write(out.join("index.json"), serde_json::to_vec_pretty(&index)?)?;
    Ok(())
}
