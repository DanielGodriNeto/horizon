//! CPU cost of each stage a column goes through, on one thread, by CPU time:
//! generation (proto, decoration, finish), first light, relight of a fixed set
//! of edits (water, light sources, opaque blocks) and meshing. Every stage
//! prints a checksum of what it made, so an A/B pair of builds can show it made
//! the same columns, light and meshes.
//!
//!     cargo run --release -p horizon-client --example cpu_bench -- \
//!         .local/horizon-client/26.3 [--seed 1234] [--center CX CZ] [--radius 4]
//!         [--rounds 5] [--edits 400]
//!
//! Each round starts from a fresh generator, so caches it fills (structure
//! starts, noise) are cold every time, as for a newly explored area. A first
//! uncounted round warms the process up. The summary gives each stage's
//! median CPU time, and its spread (max - min) over the median.
use horizon_client::{
    assets::Assets,
    biome::Biomes,
    blocks::BlockFacts,
    chunk::{Chunk, HEIGHT, MIN_Y},
    light::{LightChanges, LightTables, apply_lighting, light_chunk, relight},
    mesher::{MeshTables, Mesher, Region, borrow},
    models,
    spans::thread_cpu,
    streaming::{ChunkGenerator, Proto},
    vanillagen::VanillaTerrain,
};
use horizon_core::biome::{BiomeZoom, obfuscate_seed};
use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

type Error = Box<dyn std::error::Error>;
type Columns = HashMap<(i32, i32), Arc<Chunk>>;

fn flag<T: std::str::FromStr>(args: &[String], name: &str, default: T) -> T {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1)?.parse().ok())
        .unwrap_or(default)
}

/// The CPU time `f` takes on this thread.
fn cpu<T>(f: impl FnOnce() -> T) -> Result<(T, Duration), Error> {
    let start = thread_cpu().ok_or("no thread CPU clock here")?;
    let out = f();
    Ok((out, thread_cpu().ok_or("no thread CPU clock here")? - start))
}

fn square(center: (i32, i32), r: i32) -> Vec<(i32, i32)> {
    (center.1 - r..=center.1 + r)
        .flat_map(|z| (center.0 - r..=center.0 + r).map(move |x| (x, z)))
        .collect()
}

/// Proto chunks out to `radius + 2`, decorated out to `radius + 1`, final out
/// to `radius`: vanilla's stages, each needing its neighbours one step behind.
fn generate(
    terrain: &VanillaTerrain,
    center: (i32, i32),
    radius: i32,
    times: &mut [Duration; 3],
) -> Result<Columns, Error> {
    let mut worker = terrain.worker();
    let mut protos: HashMap<(i32, i32), Proto> = HashMap::new();
    let (made, t) = cpu(|| {
        square(center, radius + 2)
            .into_iter()
            .map(|(x, z)| ((x, z), terrain.proto(&mut worker, x, z)))
            .collect::<Vec<_>>()
    })?;
    protos.extend(made);
    times[0] += t;
    for (x, z) in square(center, radius + 1) {
        let keys: [(i32, i32); 9] =
            std::array::from_fn(|i| (x + i as i32 % 3 - 1, z + i as i32 / 3 - 1));
        let mut region: [Proto; 9] = keys.map(|k| protos.remove(&k).expect("proto"));
        times[1] += cpu(|| terrain.decorate(&mut worker, &mut region))?.1;
        for (k, p) in keys.into_iter().zip(region) {
            protos.insert(k, p);
        }
    }
    let mut columns = Columns::new();
    for key in square(center, radius) {
        let proto = protos.remove(&key).expect("proto");
        let (chunk, t) = cpu(|| terrain.finish(proto))?;
        times[2] += t;
        columns.insert(key, Arc::new(chunk));
    }
    Ok(columns)
}

fn region_of(columns: &Columns, (x, z): (i32, i32)) -> [[Option<&Chunk>; 3]; 3] {
    std::array::from_fn(|dz| {
        std::array::from_fn(|dx| {
            columns
                .get(&(x + dx as i32 - 1, z + dz as i32 - 1))
                .map(|c| &**c)
        })
    })
}

fn hash_light(columns: &Columns, keys: &[(i32, i32)], hash: &mut DefaultHasher) {
    for key in keys {
        for section in &columns[key].sections {
            (0..4096).for_each(|i| section.light(i).hash(hash));
        }
    }
}

fn main() -> Result<(), Error> {
    let args: Vec<String> = std::env::args().collect();
    let root = PathBuf::from(args.get(1).ok_or("usage: cpu_bench <resources>")?);
    let seed: i64 = flag(&args, "--seed", 1234);
    let radius: i32 = flag(&args, "--radius", 4);
    let rounds: usize = flag(&args, "--rounds", 5);
    let edits: usize = flag(&args, "--edits", 400);
    let center = args
        .iter()
        .position(|a| a == "--center")
        .map(|i| -> Result<(i32, i32), Error> {
            Ok((
                args.get(i + 1).ok_or("--center CX CZ")?.parse()?,
                args.get(i + 2).ok_or("--center CX CZ")?.parse()?,
            ))
        })
        .transpose()?
        .unwrap_or((-60, 10));

    let assets = Arc::new(Assets::load(&root)?);
    let render = Arc::new(assets.reload(&root)?);
    let facts = BlockFacts::load(&root, assets.registry)?;
    let visibility = models::load_visibility(&root, assets.registry)?;
    let biomes = Biomes::load(&root)?;
    let tables = MeshTables::new(&assets, &render, &facts, visibility)?;
    let light = LightTables::load(&root, assets.registry, &facts)?;
    let palette: Vec<u32> = [
        "minecraft:water",
        "minecraft:glowstone",
        "minecraft:air",
        "minecraft:stone",
        "minecraft:torch",
        "minecraft:lava",
    ]
    .iter()
    .filter_map(|n| assets.registry.default_state(n).map(|s| s.0))
    .collect();

    let names = [
        "gen_proto",
        "gen_decorate",
        "gen_finish",
        "light",
        "relight",
        "mesh",
    ];
    let mut samples: Vec<Vec<f64>> = vec![Vec::new(); names.len()];
    let mut sums = [0u64; 4];
    let lit_keys = square(center, radius - 1);
    let mesh_keys = square(center, radius - 2);
    // Round 0 warms up the process (page faults, lazy tables) and is not counted.
    for round in 0..=rounds.max(1) {
        let terrain = VanillaTerrain::new(&root.join("worldgen"), assets.registry, &biomes, seed)?;
        let mut gen_times = [Duration::ZERO; 3];
        let mut columns = generate(&terrain, center, radius, &mut gen_times)?;
        let mut hash = DefaultHasher::new();
        for key in square(center, radius) {
            for section in &columns[&key].sections {
                (0..4096).for_each(|i| section.get(i).hash(&mut hash));
            }
        }
        let gen_sum = hash.finish();

        // First light of every column whose neighbours exist.
        let (lights, light_time) = cpu(|| {
            lit_keys
                .iter()
                .map(|&key| (key, light_chunk(&region_of(&columns, key), &light, true)))
                .collect::<Vec<_>>()
        })?;
        for (key, lights) in lights {
            let chunk = Arc::make_mut(columns.get_mut(&key).expect("column"));
            apply_lighting(chunk, lights);
            chunk.lit = true;
        }
        columns.retain(|k, _| lit_keys.contains(k));
        let mut hash = DefaultHasher::new();
        hash_light(&columns, &lit_keys, &mut hash);
        let light_sum = hash.finish();

        // The same edits every round: a block near the surface of an inner
        // column, then the incremental relight it causes.
        let mut state = 0x2545_F491_4F6C_DD1Du64;
        let mut next = |n: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % n
        };
        let mut relight_time = Duration::ZERO;
        for _ in 0..edits {
            let (cx, cz) = mesh_keys[next(mesh_keys.len() as u64) as usize];
            let (lx, lz) = (next(16) as i32, next(16) as i32);
            let column = &columns[&(cx, cz)];
            let top = (MIN_Y..MIN_Y + HEIGHT)
                .rev()
                .find(|&y| column.get(lx, y, lz) != 0)
                .unwrap_or(64);
            let y = top - 3 + next(6) as i32;
            let block = palette[next(palette.len() as u64) as usize];
            let old =
                Arc::make_mut(columns.get_mut(&(cx, cz)).expect("column")).set(lx, y, lz, block);
            if old == block || !light.differs(old, block) {
                continue;
            }
            let at = (cx * 16 + lx, y, cz * 16 + lz);
            let mut changed = LightChanges::default();
            relight_time +=
                cpu(|| relight(&mut columns, &light, true, &[at], false, &mut changed))?.1;
        }
        let mut hash = DefaultHasher::new();
        hash_light(&columns, &lit_keys, &mut hash);
        let relight_sum = hash.finish();

        // Every section of the inner columns, as the mesh workers do.
        let mut mesher = Mesher {
            nether_shading: false,
            zoom: BiomeZoom::new(obfuscate_seed(seed)),
            blend_radius: 2,
            smooth_lighting: true,
            cutout_leaves: true,
            assets: &assets,
            render: &render,
            tables: &tables,
            biomes: &biomes,
            visibility: tables.visibility.clone(),
        };
        let mut hash = DefaultHasher::new();
        let mut mesh_time = Duration::ZERO;
        for &(x, z) in &mesh_keys {
            let region: Region = std::array::from_fn(|dz| {
                std::array::from_fn(|dx| {
                    columns
                        .get(&(x + dx as i32 - 1, z + dz as i32 - 1))
                        .cloned()
                })
            });
            let borrowed = borrow(&region);
            for sy in 0..HEIGHT / 16 {
                let (mesh, t) = cpu(|| mesher.mesh(&borrowed, sy))?;
                mesh_time += t;
                bytemuck::cast_slice::<_, u8>(&mesh.solid).hash(&mut hash);
                bytemuck::cast_slice::<_, u8>(&mesh.translucent).hash(&mut hash);
            }
        }
        let mesh_sum = hash.finish();

        let times = [
            gen_times[0],
            gen_times[1],
            gen_times[2],
            light_time,
            relight_time,
            mesh_time,
        ];
        if round > 0 {
            for (s, t) in samples.iter_mut().zip(times) {
                s.push(t.as_secs_f64() * 1e3);
            }
        }
        let now = [gen_sum, light_sum, relight_sum, mesh_sum];
        if round > 0 && now != sums {
            return Err(
                format!("round {round} made different output: {now:x?} vs {sums:x?}").into(),
            );
        }
        sums = now;
        println!(
            "round {round}: {}",
            names
                .iter()
                .zip(times)
                .map(|(n, t)| format!("{n} {:.1}", t.as_secs_f64() * 1e3))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let units = [
        ("column", square(center, radius + 2).len()),
        ("column", square(center, radius + 1).len()),
        ("column", square(center, radius).len()),
        ("column", lit_keys.len()),
        ("edit", edits),
        ("section", mesh_keys.len() * HEIGHT as usize / 16),
    ];
    println!(
        "checksums gen {:016x} light {:016x} relight {:016x} mesh {:016x}",
        sums[0], sums[1], sums[2], sums[3]
    );
    for ((name, s), (unit, n)) in names.iter().zip(&mut samples).zip(units) {
        s.sort_by(f64::total_cmp);
        let median = s[s.len() / 2];
        let spread = (s[s.len() - 1] - s[0]) / median * 100.0;
        println!(
            "stage {name:<13} median {median:9.2} ms  spread {spread:5.1} %  {:8.4} ms/{unit}",
            median / n.max(1) as f64
        );
    }
    Ok(())
}
