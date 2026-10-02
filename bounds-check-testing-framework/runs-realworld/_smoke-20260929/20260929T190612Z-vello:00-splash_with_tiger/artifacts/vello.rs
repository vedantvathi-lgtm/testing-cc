//! RW1: vello (the `vello` crate = `research/vello_research` at a9f11bb) rendering its own test
//! scenes, replayed on the wgpu fork. `vello-capture` recorded vello's exact per-frame command
//! stream (`low_level::Render`, the calls `Renderer::render_to_texture` makes) and vello's own
//! rendered image. This replays it the way vello's `WgpuEngine::run_recording` does:
//! - buffer sizes rounded to vello's size classes (`BufferPool::size_class`, 1 bit), whole-buffer
//!   bindings unless the recording binds a range, so `arrayLength()` sees what it sees in vello;
//! - uploads land before the frame's commands (vello uses `queue.write_buffer/write_texture`);
//!   here they are GPU copies from pristine staging buffers at the start of each frame;
//! - one compute pass per dispatch, zero-size dispatches skipped (vello issue #637 workaround);
//! - pending clears of not-yet-materialised buffers become clears at that point, which is
//!   equivalent because every buffer here is persistent.
//! Shaders: vello_shaders' preprocessed WGSL, layouts from vello's `full_shaders`.

use crate::compile::ShaderSpec;
use crate::workload::{read_buffer, Workload};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct Vello {
    name: String,
    dir: PathBuf,
    stem: String,
    rec: Value,
    shaders: Vec<Value>,
    blob: Vec<u8>,
    state: Option<State>,
}

enum Op {
    CopyBuf { staging: usize, buf: u64, len: u64 },
    CopyImage { staging: usize, image: u64, x: u32, y: u32, w: u32, h: u32, row: u32 },
    Clear { buf: u64, offset: u64, len: Option<u64> },
    Dispatch { shader: usize, wg: [u32; 3], group: usize },
    Indirect { shader: usize, buf: u64, offset: u64, group: usize },
}

struct State {
    buffers: BTreeMap<u64, wgpu::Buffer>,
    images: BTreeMap<u64, wgpu::Texture>,
    staging: Vec<wgpu::Buffer>,
    groups: Vec<wgpu::BindGroup>,
    pipelines: Vec<Vec<wgpu::ComputePipeline>>,
    uploads: Vec<Op>,
    ops: Vec<Op>,
    out_image: u64,
    width: u32,
    height: u32,
    readback: wgpu::Buffer,
}

fn size_class(x: u64) -> u64 {
    // vello research/vello_research/src/wgpu_engine.rs `size_class`, SIZE_CLASS_BITS = 1
    let bits = 1;
    if x > 1 << bits {
        let a = (x - 1).leading_zeros();
        let b = (x - 1) | (((u64::MAX / 2) >> bits) >> a);
        b + 1
    } else {
        1 << bits
    }
}

fn format(s: &str) -> wgpu::TextureFormat {
    match s {
        "rgba8" => wgpu::TextureFormat::Rgba8Unorm,
        "bgra8" => wgpu::TextureFormat::Bgra8Unorm,
        _ => panic!("image format {s}"),
    }
}

impl Vello {
    pub fn load(dir: &Path, stem: &str) -> Self {
        let rec: Value = serde_json::from_slice(&std::fs::read(dir.join(format!("{stem}.json"))).expect("capture json")).unwrap();
        let shaders_file: Value = serde_json::from_slice(&std::fs::read(dir.join("shaders.json")).unwrap()).unwrap();
        let blob = std::fs::read(dir.join(format!("{stem}.blob"))).unwrap();
        Vello { name: format!("vello:{stem}"), dir: dir.to_path_buf(), stem: stem.into(), rec, shaders: shaders_file["shaders"].as_array().unwrap().clone(), blob, state: None }
    }

    pub fn stems(dir: &Path) -> Vec<String> {
        let index: Value = match std::fs::read(dir.join("index.json")) {
            Ok(b) => serde_json::from_slice(&b).unwrap(),
            Err(_) => return Vec::new(),
        };
        index.as_array().unwrap().iter().filter_map(|e| e["stem"].as_str().map(str::to_string)).collect()
    }

    fn data(&self, v: &Value) -> &[u8] {
        let (off, len) = (v[0].as_u64().unwrap() as usize, v[1].as_u64().unwrap() as usize);
        &self.blob[off..off + len]
    }

    fn frame(&self, s: &State, slot: usize, encoder: &mut wgpu::CommandEncoder) {
        for op in s.uploads.iter().chain(&s.ops) {
            self.run_op(s, slot, op, encoder);
        }
    }

    fn run_op(&self, s: &State, slot: usize, op: &Op, encoder: &mut wgpu::CommandEncoder) {
        {
            match *op {
                Op::CopyBuf { staging, buf, len } => encoder.copy_buffer_to_buffer(&s.staging[staging], 0, &s.buffers[&buf], 0, len),
                Op::CopyImage { staging, image, x, y, w, h, row } => encoder.copy_buffer_to_texture(
                    wgpu::TexelCopyBufferInfo { buffer: &s.staging[staging], layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: None } },
                    wgpu::TexelCopyTextureInfo { texture: &s.images[&image], mip_level: 0, origin: wgpu::Origin3d { x, y, z: 0 }, aspect: wgpu::TextureAspect::All },
                    wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                ),
                Op::Clear { buf, offset, len } => encoder.clear_buffer(&s.buffers[&buf], offset, len),
                Op::Dispatch { shader, wg, group } => {
                    let mut pass = encoder.begin_compute_pass(&Default::default());
                    pass.set_pipeline(&s.pipelines[slot][shader]);
                    pass.set_bind_group(0, &s.groups[group], &[]);
                    pass.dispatch_workgroups(wg[0], wg[1], wg[2]);
                }
                Op::Indirect { shader, buf, offset, group } => {
                    let mut pass = encoder.begin_compute_pass(&Default::default());
                    pass.set_pipeline(&s.pipelines[slot][shader]);
                    pass.set_bind_group(0, &s.groups[group], &[]);
                    pass.dispatch_workgroups_indirect(&s.buffers[&buf], offset);
                }
            }
        }
    }
}

impl Workload for Vello {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn provenance(&self) -> serde_json::Value {
        serde_json::json!({
            "renderer": "vello (crate `vello` = research/vello_research), compute pipeline",
            "vello_rev": self.rec["vello_rev"], "scene_index": self.rec["scene_index"], "scene": self.rec["scene"],
            "width": self.rec["width"], "height": self.rec["height"], "aa": self.rec["aa"],
            "capture": format!("{}/{}.json", self.dir.display(), self.stem),
            "capture_sha256": crate::compile::sha256(&std::fs::read(self.dir.join(format!("{}.json", self.stem))).unwrap()),
            "blob_sha256": crate::compile::sha256(&self.blob),
            "note": "native vello compiles these shaders with ShaderRuntimeChecks::unchecked(); the checked slots model vello as web content in a wgpu-based browser",
        })
    }

    fn shaders(&self) -> Vec<ShaderSpec> {
        self.shaders
            .iter()
            .map(|s| {
                let n = s["layout"].as_array().unwrap().len() as u64;
                let idx: Vec<u64> = s["wgsl_binding_indices"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect();
                assert_eq!(idx, (0..n).collect::<Vec<_>>(), "{}: WGSL bindings must be 0..n so wgpu-hal's rank map is the identity", s["field"]);
                ShaderSpec { label: Box::leak(s["field"].as_str().unwrap().to_string().into_boxed_str()), wgsl: s["wgsl"].as_str().unwrap().into(), entry: "main", stage: naga::ShaderStage::Compute }
            })
            .collect()
    }

    fn setup(&mut self, device: &wgpu::Device, _queue: &wgpu::Queue, modules: &[Vec<wgpu::ShaderModule>]) {
        use wgpu::util::DeviceExt;
        let cs = wgpu::ShaderStages::COMPUTE;
        let layouts: Vec<wgpu::BindGroupLayout> = self
            .shaders
            .iter()
            .map(|s| {
                let entries: Vec<wgpu::BindGroupLayoutEntry> = s["layout"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .enumerate()
                    .map(|(i, t)| {
                        let ty = match t.as_str().unwrap() {
                            "buffer" => wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None },
                            "buffer_read_only" => wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                            "uniform" => wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                            f if f.starts_with("image_read_") => wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                            f if f.starts_with("image_") => wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format: format(&f["image_".len()..]), view_dimension: wgpu::TextureViewDimension::D2 },
                            other => panic!("bind type {other}"),
                        };
                        wgpu::BindGroupLayoutEntry { binding: i as u32, visibility: cs, ty, count: None }
                    })
                    .collect();
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: s["field"].as_str(), entries: &entries })
            })
            .collect();
        let pipelines: Vec<Vec<wgpu::ComputePipeline>> = modules
            .iter()
            .map(|slot_modules| {
                slot_modules
                    .iter()
                    .zip(&layouts)
                    .zip(&self.shaders)
                    .map(|((m, l), s)| {
                        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(l)], immediate_size: 0 });
                        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                            label: s["field"].as_str(),
                            layout: Some(&pl),
                            module: m,
                            entry_point: Some("main"),
                            // vello passes zero_initialize_workgroup_memory: false
                            compilation_options: wgpu::PipelineCompilationOptions { zero_initialize_workgroup_memory: false, ..Default::default() },
                            cache: None,
                        })
                    })
                    .collect()
            })
            .collect();

        let out = &self.rec["out_image"];
        let out_image = out["id"].as_u64().unwrap();
        let (width, height) = (out["width"].as_u64().unwrap() as u32, out["height"].as_u64().unwrap() as u32);
        let mut buffers: BTreeMap<u64, wgpu::Buffer> = BTreeMap::new();
        let mut images: BTreeMap<u64, wgpu::Texture> = BTreeMap::new();
        let storage = wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT;
        let buffer = |buffers: &mut BTreeMap<u64, wgpu::Buffer>, b: &Value, usage: wgpu::BufferUsages| -> u64 {
            let id = b["id"].as_u64().unwrap();
            buffers.entry(id).or_insert_with(|| {
                device.create_buffer(&wgpu::BufferDescriptor { label: b["name"].as_str(), size: size_class(b["size"].as_u64().unwrap()), usage, mapped_at_creation: false })
            });
            id
        };
        let image = |images: &mut BTreeMap<u64, wgpu::Texture>, i: &Value| -> u64 {
            let id = i["id"].as_u64().unwrap();
            images.entry(id).or_insert_with(|| {
                let usage = if id == out_image {
                    wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST
                } else {
                    wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST
                };
                device.create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d { width: i["width"].as_u64().unwrap() as u32, height: i["height"].as_u64().unwrap() as u32, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: format(i["format"].as_str().unwrap()),
                    usage,
                    view_formats: &[],
                })
            });
            id
        };
        let mut staging = Vec::new();
        let mut uploads = Vec::new();
        let mut ops = Vec::new();
        let mut groups = Vec::new();
        let padded_image = |data: &[u8], w: u32, h: u32| -> (Vec<u8>, u32) {
            let row = w * 4;
            let padded = row.next_multiple_of(256);
            let mut out = vec![0u8; (padded * h) as usize];
            for y in 0..h as usize {
                out[y * padded as usize..y * padded as usize + row as usize].copy_from_slice(&data[y * row as usize..(y + 1) * row as usize]);
            }
            (out, padded)
        };
        for c in self.rec["commands"].as_array().unwrap() {
            match c["op"].as_str().unwrap() {
                op @ ("upload" | "upload_uniform") => {
                    let usage = if op == "upload" { storage } else { wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST };
                    let id = buffer(&mut buffers, &c["buf"], usage);
                    let mut data = self.data(&c["data"]).to_vec();
                    data.resize(data.len().next_multiple_of(4), 0);
                    let len = data.len() as u64;
                    staging.push(device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("staging"), contents: &data, usage: wgpu::BufferUsages::COPY_SRC }));
                    uploads.push(Op::CopyBuf { staging: staging.len() - 1, buf: id, len });
                }
                op @ ("upload_image" | "write_image") => {
                    let id = image(&mut images, &c["image"]);
                    let (w, h, x, y) = if op == "upload_image" {
                        (c["image"]["width"].as_u64().unwrap() as u32, c["image"]["height"].as_u64().unwrap() as u32, 0, 0)
                    } else {
                        (c["width"].as_u64().unwrap() as u32, c["height"].as_u64().unwrap() as u32, c["x"].as_u64().unwrap() as u32, c["y"].as_u64().unwrap() as u32)
                    };
                    if w == 0 || h == 0 {
                        continue;
                    }
                    let (data, row) = padded_image(self.data(&c["data"]), w, h);
                    staging.push(device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("staging-image"), contents: &data, usage: wgpu::BufferUsages::COPY_SRC }));
                    uploads.push(Op::CopyImage { staging: staging.len() - 1, image: id, x, y, w, h, row });
                }
                "clear" => {
                    let id = buffer(&mut buffers, &c["buf"], storage);
                    ops.push(Op::Clear { buf: id, offset: c["offset"].as_u64().unwrap(), len: c["len"].as_u64() });
                }
                "download" => panic!("download in a render recording"),
                "free_buffer" | "free_image" => {}
                op @ ("dispatch" | "dispatch_indirect") => {
                    let shader = c["shader"].as_u64().unwrap() as usize;
                    if op == "dispatch" {
                        let wg: Vec<u32> = c["wg"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as u32).collect();
                        if wg.contains(&0) {
                            continue;
                        }
                    }
                    let mut views = Vec::new();
                    let mut entries_spec = Vec::new();
                    for (i, b) in c["bindings"].as_array().unwrap().iter().enumerate() {
                        match b["kind"].as_str().unwrap() {
                            "buffer" => entries_spec.push((i, buffer(&mut buffers, b, storage), None)),
                            "range" => entries_spec.push((i, buffer(&mut buffers, b, storage), Some((b["offset"].as_u64().unwrap(), b["range_size"].as_u64().unwrap())))),
                            _ => {
                                let id = image(&mut images, &b["image"]);
                                views.push((i, images[&id].create_view(&wgpu::TextureViewDescriptor::default())));
                            }
                        }
                    }
                    let mut entries: Vec<wgpu::BindGroupEntry> = entries_spec
                        .iter()
                        .map(|(i, id, range)| wgpu::BindGroupEntry {
                            binding: *i as u32,
                            resource: match range {
                                None => buffers[id].as_entire_binding(),
                                Some((offset, size)) => wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &buffers[id], offset: *offset, size: std::num::NonZeroU64::new(*size) }),
                            },
                        })
                        .collect();
                    entries.extend(views.iter().map(|(i, v)| wgpu::BindGroupEntry { binding: *i as u32, resource: wgpu::BindingResource::TextureView(v) }));
                    entries.sort_by_key(|e| e.binding);
                    groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &layouts[shader], entries: &entries }));
                    let group = groups.len() - 1;
                    if op == "dispatch" {
                        let wg: Vec<u32> = c["wg"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as u32).collect();
                        ops.push(Op::Dispatch { shader, wg: [wg[0], wg[1], wg[2]], group });
                    } else {
                        let id = buffer(&mut buffers, &c["buf"], storage);
                        ops.push(Op::Indirect { shader, buf: id, offset: c["offset"].as_u64().unwrap(), group });
                    }
                }
                other => panic!("unknown op {other}"),
            }
        }
        let padded = (width * 4).next_multiple_of(256);
        let readback = device.create_buffer(&wgpu::BufferDescriptor { label: Some("readback"), size: u64::from(padded * height), usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
        self.state = Some(State { buffers, images, staging, groups, pipelines, uploads, ops, out_image, width, height, readback });
    }

    fn record(&self, slot: usize, encoder: &mut wgpu::CommandEncoder, queries: &wgpu::QuerySet, reps: u32) {
        let s = self.state.as_ref().unwrap();
        // Empty passes carry the timestamps so the window covers every command of every frame.
        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: Some(wgpu::ComputePassTimestampWrites { query_set: queries, beginning_of_pass_write_index: Some(0), end_of_pass_write_index: None }) });
        for _ in 0..reps {
            self.frame(s, slot, encoder);
        }
        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: Some(wgpu::ComputePassTimestampWrites { query_set: queries, beginning_of_pass_write_index: None, end_of_pass_write_index: Some(1) }) });
    }

    fn reference_outputs(&self) -> Vec<Vec<u8>> {
        references(&self.dir, &self.stem)
    }

    fn output(&self, device: &wgpu::Device, queue: &wgpu::Queue, slot: usize) -> Vec<u8> {
        let s = self.state.as_ref().unwrap();
        let zeros = vec![0u8; (s.width * s.height * 4) as usize];
        let out = &s.images[&s.out_image];
        let copy = wgpu::TexelCopyTextureInfo { texture: out, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All };
        let size = wgpu::Extent3d { width: s.width, height: s.height, depth_or_array_layers: 1 };
        queue.write_texture(copy, &zeros, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(s.width * 4), rows_per_image: None }, size);
        let mut encoder = device.create_command_encoder(&Default::default());
        if std::env::var("BCE_VELLO_FRESH").is_ok() {
            // Diagnostic only: start the frame from all-zero buffers.
            for b in s.buffers.values() {
                if b.usage().contains(wgpu::BufferUsages::COPY_DST) && b.usage().contains(wgpu::BufferUsages::STORAGE) {
                    encoder.clear_buffer(b, 0, None);
                }
            }
        }
        if std::env::var("BCE_VELLO_SPLIT_SUBMIT").is_ok() {
            // Diagnostic only: every upload and op in its own submission.
            for op in s.uploads.iter().chain(&s.ops) {
                let mut e = device.create_command_encoder(&Default::default());
                self.run_op(s, slot, op, &mut e);
                queue.submit([e.finish()]);
                device.poll(wgpu::PollType::wait_indefinitely()).expect("poll");
            }
        } else {
            self.frame(s, slot, &mut encoder);
        }
        let padded = (s.width * 4).next_multiple_of(256);
        encoder.copy_texture_to_buffer(copy, wgpu::TexelCopyBufferInfo { buffer: &s.readback, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: None } }, size);
        queue.submit([encoder.finish()]);
        let data = read_buffer(device, &s.readback);
        data.chunks(padded as usize).flat_map(|r| r[..(s.width * 4) as usize].to_vec()).collect()
    }
}

/// Every distinct image vello's own renderer produced for this scene (see vello-capture).
pub fn references(dir: &Path, stem: &str) -> Vec<Vec<u8>> {
    let meta: Value = match std::fs::read(dir.join(format!("{stem}.refs.json"))) {
        Ok(b) => serde_json::from_slice(&b).unwrap(),
        Err(_) => return Vec::new(),
    };
    meta["states"].as_array().unwrap().iter().map(|st| std::fs::read(dir.join(st["file"].as_str().unwrap())).unwrap()).collect()
}
