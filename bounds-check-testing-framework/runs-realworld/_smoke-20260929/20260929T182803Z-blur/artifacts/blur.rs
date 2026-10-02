//! RW2: the WebGPU samples' image blur (`sample/imageBlur/blur.wgsl`, unmodified), driven exactly
//! like `sample/imageBlur/main.ts` `frame()`: filterSize 15 (filterDim 16, blockDim 113),
//! 2 iterations = 4 dispatches alternating `flip`, on the sample's own image `Di-3d.png`.

use crate::compile::ShaderSpec;
use crate::workload::{read_buffer, Workload};
use std::path::Path;

const TILE_DIM: u32 = 128;
const BATCH: u32 = 4;

pub struct Blur {
    name: &'static str,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    tile: u32,
    filter_size: u32,
    iterations: u32,
    samples: std::path::PathBuf,
    state: Option<State>,
}

struct State {
    pipelines: Vec<wgpu::ComputePipeline>,
    constants: wgpu::BindGroup,
    groups: [wgpu::BindGroup; 3],
    image: wgpu::Texture,
    textures: [wgpu::Texture; 2],
    readback: wgpu::Buffer,
    padded_row: u32,
}

impl Blur {
    /// `tile` > 1 repeats the sample image `tile` x `tile` times (a larger frame, same content).
    pub fn new(name: &'static str, project: &Path, tile: u32) -> Self {
        let samples = project.join("references/raw/webgpu-samples");
        let inputs = project.join("bounds-check-testing-framework/realworld-v1/inputs");
        let meta: serde_json::Value = serde_json::from_slice(&std::fs::read(inputs.join("Di-3d.json")).expect("run inputs/prepare.sh first")).unwrap();
        let (w, h) = (meta["width"].as_u64().unwrap() as u32, meta["height"].as_u64().unwrap() as u32);
        let src = std::fs::read(inputs.join("Di-3d.rgba")).unwrap();
        assert_eq!(src.len(), (w * h * 4) as usize);
        let (width, height) = (w * tile, h * tile);
        let mut rgba = vec![0u8; (width * height * 4) as usize];
        for y in 0..height {
            for x in 0..width {
                let s = (((y % h) * w + (x % w)) * 4) as usize;
                let d = ((y * width + x) * 4) as usize;
                rgba[d..d + 4].copy_from_slice(&src[s..s + 4]);
            }
        }
        Blur { name, width, height, rgba, tile, filter_size: 15, iterations: 2, samples, state: None }
    }

    fn block_dim(&self) -> u32 {
        TILE_DIM - self.filter_size
    }

    fn frame(&self, s: &State, pass: &mut wgpu::ComputePass, slot: usize) {
        let bd = self.block_dim();
        let a = (self.width.div_ceil(bd), self.height.div_ceil(BATCH));
        let b = (self.height.div_ceil(bd), self.width.div_ceil(BATCH));
        pass.set_pipeline(&s.pipelines[slot]);
        pass.set_bind_group(0, &s.constants, &[]);
        pass.set_bind_group(1, &s.groups[0], &[]);
        pass.dispatch_workgroups(a.0, a.1, 1);
        pass.set_bind_group(1, &s.groups[1], &[]);
        pass.dispatch_workgroups(b.0, b.1, 1);
        for _ in 0..self.iterations - 1 {
            pass.set_bind_group(1, &s.groups[2], &[]);
            pass.dispatch_workgroups(a.0, a.1, 1);
            pass.set_bind_group(1, &s.groups[1], &[]);
            pass.dispatch_workgroups(b.0, b.1, 1);
        }
    }

    fn texture_copy<'a>(texture: &'a wgpu::Texture) -> wgpu::TexelCopyTextureInfo<'a> {
        wgpu::TexelCopyTextureInfo { texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All }
    }

    fn write_rgba(&self, queue: &wgpu::Queue, texture: &wgpu::Texture, data: &[u8]) {
        queue.write_texture(
            Self::texture_copy(texture),
            data,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(self.width * 4), rows_per_image: Some(self.height) },
            wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
        );
    }
}

impl Workload for Blur {
    fn name(&self) -> String {
        self.name.into()
    }

    fn provenance(&self) -> serde_json::Value {
        serde_json::json!({
            "shader": "webgpu-samples sample/imageBlur/blur.wgsl (unmodified)",
            "host_logic": "webgpu-samples sample/imageBlur/main.ts frame(): filterSize 15, iterations 2",
            "input": "webgpu-samples public/assets/img/Di-3d.png",
            "tile": self.tile, "width": self.width, "height": self.height,
            "filter_size": self.filter_size, "iterations": self.iterations,
            "webgpu_samples_rev": crate::git(&self.samples, &["rev-parse", "HEAD"]),
        })
    }

    fn shaders(&self) -> Vec<ShaderSpec> {
        let wgsl = std::fs::read_to_string(self.samples.join("sample/imageBlur/blur.wgsl")).expect("blur.wgsl");
        vec![ShaderSpec { label: "blur", wgsl, entry: "main", stage: naga::ShaderStage::Compute }]
    }

    fn setup(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, modules: &[Vec<wgpu::ShaderModule>]) {
        use wgpu::util::DeviceExt;
        let size = wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 };
        let texture = |label, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage,
                view_formats: &[],
            })
        };
        let image = texture("image", wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST);
        self.write_rgba(queue, &image, &self.rgba);
        let work = wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING;
        let textures = [texture("t0", work), texture("t1", work)];
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor { mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..Default::default() });
        let uniform = |label, words: &[u32]| device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some(label), contents: bytemuck::cast_slice(words), usage: wgpu::BufferUsages::UNIFORM });
        let params = uniform("params", &[self.filter_size + 1, self.block_dim()]);
        let flip0 = uniform("flip0", &[0]);
        let flip1 = uniform("flip1", &[1]);
        let cs = wgpu::ShaderStages::COMPUTE;
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry { binding, visibility: cs, ty, count: None };
        let uniform_ty = wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None };
        let layout0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("constants"),
            entries: &[entry(0, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)), entry(1, uniform_ty)],
        });
        let layout1 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("io"),
            entries: &[
                entry(1, wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }),
                entry(2, wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format: wgpu::TextureFormat::Rgba8Unorm, view_dimension: wgpu::TextureViewDimension::D2 }),
                entry(3, uniform_ty),
            ],
        });
        let constants = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("constants"),
            layout: &layout0,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Sampler(&sampler) },
                wgpu::BindGroupEntry { binding: 1, resource: params.as_entire_binding() },
            ],
        });
        let view = |t: &wgpu::Texture| t.create_view(&wgpu::TextureViewDescriptor::default());
        let (vi, v0, v1) = (view(&image), view(&textures[0]), view(&textures[1]));
        let group = |src: &wgpu::TextureView, dst: &wgpu::TextureView, flip: &wgpu::Buffer| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("io"),
                layout: &layout1,
                entries: &[
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(src) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(dst) },
                    wgpu::BindGroupEntry { binding: 3, resource: flip.as_entire_binding() },
                ],
            })
        };
        let groups = [group(&vi, &v0, &flip0), group(&v0, &v1, &flip1), group(&v1, &v0, &flip0)];
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("blur"), bind_group_layouts: &[Some(&layout0), Some(&layout1)], immediate_size: 0 });
        let pipelines = modules
            .iter()
            .map(|m| {
                device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("blur"),
                    layout: Some(&pipeline_layout),
                    module: &m[0],
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    cache: None,
                })
            })
            .collect();
        let padded_row = (self.width * 4).div_ceil(256) * 256;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(padded_row * self.height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        self.state = Some(State { pipelines, constants, groups, image, textures, readback, padded_row });
    }

    fn record(&self, slot: usize, encoder: &mut wgpu::CommandEncoder, queries: &wgpu::QuerySet, reps: u32) {
        let s = self.state.as_ref().unwrap();
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: Some(wgpu::ComputePassTimestampWrites { query_set: queries, beginning_of_pass_write_index: Some(0), end_of_pass_write_index: Some(1) }),
        });
        for _ in 0..reps {
            self.frame(s, &mut pass, slot);
        }
    }

    fn output(&self, device: &wgpu::Device, queue: &wgpu::Queue, slot: usize) -> Vec<u8> {
        let s = self.state.as_ref().unwrap();
        let zeros = vec![0u8; self.rgba.len()];
        for t in &s.textures {
            self.write_rgba(queue, t, &zeros);
        }
        let _ = &s.image;
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            self.frame(s, &mut pass, slot);
        }
        encoder.copy_texture_to_buffer(
            Self::texture_copy(&s.textures[1]),
            wgpu::TexelCopyBufferInfo {
                buffer: &s.readback,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(s.padded_row), rows_per_image: Some(self.height) },
            },
            wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
        );
        queue.submit([encoder.finish()]);
        let padded = read_buffer(device, &s.readback);
        let row = (self.width * 4) as usize;
        padded.chunks(s.padded_row as usize).flat_map(|r| r[..row].to_vec()).collect()
    }
}
