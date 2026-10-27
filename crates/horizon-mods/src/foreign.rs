//! Foreign mod jars (a `[turbo]` experiment, feature
//! `foreign`): recognises the loader a jar was made for, and adapts a Fabric
//! or Quilt jar, or a datapack, that ships no code into a content mod
//! ([`crate::Content`]). Nothing is extracted: the jar is read as a zip, a few
//! small metadata entries at a time, and its files are read in place.
//! Jars with Java code, and NeoForge, Forge and legacy Forge mods, are refused:
//! this build has no JVM.
use crate::manifest::{Manifest, Package};
use serde_json::Value;
use std::{collections::BTreeSet, fmt, fs::File, io::Read, path::Path};
use zip::ZipArchive;

/// The biggest metadata entry read from a jar; the real ones are a few KiB.
const MAX_ENTRY: u64 = 1 << 20;
/// The game's and the convention tags' namespaces: a jar may add to their
/// tags, but it does not claim them.
const SHARED: [&str; 2] = ["minecraft", "c"];
const NEOFORGE_TOML: &str = "META-INF/neoforge.mods.toml";
const FORGE_TOML: &str = "META-INF/mods.toml";
/// The marker files, in precedence order.
const MARKERS: [(&str, Loader); 5] = [
    ("fabric.mod.json", Loader::Fabric),
    ("quilt.mod.json", Loader::Quilt),
    (NEOFORGE_TOML, Loader::NeoForge),
    (FORGE_TOML, Loader::Forge),
    ("mcmod.info", Loader::ForgeLegacy),
];

type Jar = ZipArchive<File>;

/// The loader a jar was made for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loader {
    Fabric,
    Quilt,
    NeoForge,
    Forge,
    ForgeLegacy,
    DataPack,
    ResourcePack,
    Unknown,
}

impl fmt::Display for Loader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Loader::Fabric => "Fabric",
            Loader::Quilt => "Quilt",
            Loader::NeoForge => "NeoForge",
            Loader::Forge => "Forge",
            Loader::ForgeLegacy => "Forge (legacy)",
            Loader::DataPack => "datapack",
            Loader::ResourcePack => "resource pack",
            Loader::Unknown => "unknown",
        })
    }
}

/// What a jar says about itself. `version` may be empty; `namespaces` are the
/// `data/` and `assets/` namespaces it fills, the game's own included.
#[derive(Clone, Debug)]
pub struct Detected {
    pub loader: Loader,
    pub id: String,
    pub name: String,
    pub version: String,
    /// `*`, `client`, `server`, or empty.
    pub environment: String,
    /// A `.class` outside `META-INF/`.
    pub has_code: bool,
    pub mixin_configs: usize,
    pub namespaces: Vec<String>,
}

/// Reads the loader and metadata of the jar at `path`, without extracting it.
pub fn detect(path: &Path) -> Result<Detected, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut jar = ZipArchive::new(file).map_err(|e| e.to_string())?;
    let Scan {
        has_code,
        has_data,
        namespaces,
    } = scan(&mut jar)?;
    let stem = path
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    let mut d = Detected {
        loader: Loader::Unknown,
        id: stem.clone(),
        name: stem,
        version: String::new(),
        environment: String::new(),
        has_code,
        mixin_configs: 0,
        namespaces,
    };
    let marker = MARKERS
        .iter()
        .find(|(name, _)| jar.by_name(name).is_ok())
        .copied();
    if let Some((name, loader)) = marker {
        d.loader = loader;
        let text = entry_text(&mut jar, name)?;
        match loader {
            Loader::Fabric => fabric(&mut d, &text)?,
            Loader::Quilt => quilt(&mut d, &text)?,
            Loader::NeoForge | Loader::Forge => forge(&mut d, &text)?,
            _ => legacy_forge(&mut d, &text),
        }
    } else if jar.by_name("pack.mcmeta").is_ok() {
        d.loader = if has_data {
            Loader::DataPack
        } else if !d.namespaces.is_empty() {
            Loader::ResourcePack
        } else {
            Loader::Unknown
        };
    }
    Ok(d)
}

/// The Horizon mod a foreign jar stands for: a content mod with no code,
/// checked by [`Manifest::parse`]. Refuses what this build cannot run.
pub fn package(path: &Path) -> Result<Package, String> {
    let d = detect(path)?;
    match d.loader {
        Loader::Fabric | Loader::Quilt if d.has_code => {
            return Err(format!(
                "{} mod {} has Java code: it needs the JVM sidecar, which this build does not have",
                d.loader, d.id
            ));
        }
        Loader::Fabric | Loader::Quilt | Loader::DataPack | Loader::ResourcePack => {}
        Loader::Unknown => return Err("not a Fabric or Quilt mod, or a datapack".into()),
        other => {
            return Err(format!(
                "{other} mods are not supported (only Fabric data mods)"
            ));
        }
    }
    let id = horizon_id(&d.id);
    // The id owns its namespace's files, so it may not be a game namespace.
    if ["minecraft", "c", "horizon"].contains(&id.as_str()) {
        return Err(format!("mod id {id} is a namespace the game keeps"));
    }
    let namespaces: Vec<String> = d
        .namespaces
        .iter()
        .filter(|ns| !SHARED.contains(&ns.as_str()) && **ns != id && namespace_ok(ns))
        .cloned()
        .collect();
    let version = if d.version.is_empty() {
        "0"
    } else {
        &d.version
    };
    let text = format!(
        "id = {}\nname = {}\nversion = {}\napi = \"0.2\"\nside = \"both\"\n\
         capabilities = [\"registries\"]\nnamespaces = {}\n",
        quote(&id),
        quote(&d.name),
        quote(version),
        Value::from(namespaces)
    );
    Ok(Package {
        manifest: Manifest::parse(&text)?,
        wasm: None,
    })
}

struct Scan {
    has_code: bool,
    has_data: bool,
    namespaces: Vec<String>,
}

/// Walks the entries once. Entries whose names leave the jar (`..`, absolute)
/// count for nothing.
fn scan(jar: &mut Jar) -> Result<Scan, String> {
    let mut scan = Scan {
        has_code: false,
        has_data: false,
        namespaces: Vec::new(),
    };
    let mut namespaces = BTreeSet::new();
    for i in 0..jar.len() {
        let entry = jar.by_index(i).map_err(|e| e.to_string())?;
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        if !entry.is_file() {
            continue;
        }
        let mut parts = name.components().map(|c| c.as_os_str().to_string_lossy());
        let (root, ns, rest) = (parts.next(), parts.next(), parts.next());
        let Some(root) = root else { continue };
        // Code anywhere but META-INF counts, `data/x/Mod.class` included.
        if root != "META-INF" && name.extension().is_some_and(|e| e == "class") {
            scan.has_code = true;
        } else if let (Some(ns), Some(_)) = (ns, rest)
            && (root == "data" || root == "assets")
        {
            scan.has_data |= root == "data";
            namespaces.insert(ns.into_owned());
        }
    }
    scan.namespaces = namespaces.into_iter().collect();
    Ok(scan)
}

/// A small text entry; the real ones are a few KiB.
fn entry_text(jar: &mut Jar, name: &str) -> Result<String, String> {
    let entry = jar.by_name(name).map_err(|e| format!("{name}: {e}"))?;
    let mut bytes = Vec::new();
    entry
        .take(MAX_ENTRY + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{name}: {e}"))?;
    if bytes.len() as u64 > MAX_ENTRY {
        return Err(format!("{name}: bigger than {MAX_ENTRY} bytes"));
    }
    String::from_utf8(bytes).map_err(|_| format!("{name}: not UTF-8"))
}

fn str_of<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key)?.as_str().filter(|s| !s.is_empty())
}

fn fabric(d: &mut Detected, text: &str) -> Result<(), String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("fabric.mod.json: {e}"))?;
    d.id = str_of(&v, "id").ok_or("fabric.mod.json: no id")?.to_owned();
    d.name = str_of(&v, "name").unwrap_or(&d.id).to_owned();
    d.version = str_of(&v, "version").unwrap_or_default().to_owned();
    d.environment = str_of(&v, "environment").unwrap_or_default().to_owned();
    d.mixin_configs = v
        .get("mixins")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    Ok(())
}

fn quilt(d: &mut Detected, text: &str) -> Result<(), String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("quilt.mod.json: {e}"))?;
    let loader = v
        .get("quilt_loader")
        .ok_or("quilt.mod.json: no quilt_loader")?;
    d.id = str_of(loader, "id")
        .ok_or("quilt.mod.json: no id")?
        .to_owned();
    d.version = str_of(loader, "version").unwrap_or_default().to_owned();
    d.name = loader
        .get("metadata")
        .and_then(|m| str_of(m, "name"))
        .unwrap_or(&d.id)
        .to_owned();
    d.environment = v
        .pointer("/minecraft/environment")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    d.mixin_configs = match v.get("mixin") {
        Some(Value::Array(a)) => a.len(),
        Some(Value::String(_)) => 1,
        _ => 0,
    };
    Ok(())
}

/// NeoForge and Forge: the first `[[mods]]` table of the toml.
fn forge(d: &mut Detected, text: &str) -> Result<(), String> {
    let toml: toml::Table = toml::from_str(text).map_err(|e| format!("mods.toml: {e}"))?;
    let first = toml
        .get("mods")
        .and_then(|m| m.as_array())
        .and_then(|a| a.first())
        .ok_or("mods.toml: no [[mods]]")?;
    let field = |k: &str| {
        first
            .get(k)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
    };
    d.id = field("modId").ok_or("mods.toml: no modId")?.to_owned();
    d.name = field("displayName").unwrap_or(&d.id).to_owned();
    d.version = field("version").unwrap_or_default().to_owned();
    Ok(())
}

/// `mcmod.info`: a list of mods, or `{"modList": [...]}`. Best effort.
fn legacy_forge(d: &mut Detected, text: &str) {
    let Ok(v) = serde_json::from_str::<Value>(text) else {
        return;
    };
    let list = v.get("modList").unwrap_or(&v);
    let Some(first) = list.as_array().and_then(|a| a.first()) else {
        return;
    };
    if let Some(id) = str_of(first, "modid") {
        d.id = id.to_owned();
    }
    d.name = str_of(first, "name").unwrap_or(&d.id).to_owned();
    d.version = str_of(first, "version").unwrap_or_default().to_owned();
}

/// Horizon's `[a-z0-9_]{1,64}`: lower case, every other character becomes `_`.
fn horizon_id(id: &str) -> String {
    id.chars()
        .take(64)
        .map(|c| match c.to_ascii_lowercase() {
            c @ ('a'..='z' | '0'..='9' | '_') => c,
            _ => '_',
        })
        .collect()
}

/// The characters [`Manifest::parse`] allows in a namespace.
fn namespace_ok(ns: &str) -> bool {
    (1..=64).contains(&ns.len())
        && ns
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b))
}

/// A TOML basic string (JSON's escapes are a subset of TOML's).
fn quote(s: &str) -> String {
    Value::from(s).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Content;
    use std::{
        io::Write,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    const FABRIC: &[u8] = br#"{"schemaVersion": 1, "id": "my-data-mod", "version": "1.0.0",
        "name": "My Data Mod", "environment": "*"}"#;
    const NEOFORGE: &[u8] = b"modLoader = \"javafml\"\n[[mods]]\nmodId = \"neo_thing\"\n\
        version = \"1.0\"\ndisplayName = \"Neo Thing\"\n";
    const PACK: &[u8] = br#"{"pack": {"pack_format": 48, "description": "a pack"}}"#;
    const RECIPE: &[u8] = b"{}";

    /// A scratch folder, removed when the test ends.
    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let dir =
                std::env::temp_dir().join(format!("horizon-foreign-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A zip at `dir/name` holding `files` (entry name, bytes).
    fn write_jar(dir: &Path, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
        let path = dir.join(name);
        let mut zip = zip::ZipWriter::new(File::create(&path).unwrap());
        for (entry, bytes) in files {
            zip.start_file(*entry, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
        path
    }

    #[test]
    fn fabric_data_jar_is_a_content_mod() {
        let dir = Scratch::new();
        let jar = write_jar(
            &dir.0,
            "my-data-mod.jar",
            &[
                ("fabric.mod.json", FABRIC),
                ("data/my-data-mod/recipe/a.json", RECIPE),
                ("data/c/tags/item/x.json", RECIPE),
                ("assets/my-data-mod/lang/en_us.json", RECIPE),
            ],
        );
        let d = detect(&jar).unwrap();
        assert_eq!(d.loader, Loader::Fabric);
        assert!(!d.has_code);
        assert_eq!(d.namespaces, ["c", "my-data-mod"]);
        let pkg = package(&jar).unwrap();
        assert_eq!(pkg.manifest.id, "my_data_mod");
        assert_eq!(pkg.manifest.namespaces, ["my-data-mod"]);
        assert!(pkg.wasm.is_none());
        let found = crate::find(&dir.0);
        assert_eq!(found, [jar]);
        let read = Package::read(&found[0]).unwrap();
        let content = Content::read(&found[0], &read.manifest).unwrap();
        assert_eq!(
            content.files,
            [
                "assets/my-data-mod/lang/en_us.json",
                "data/c/tags/item/x.json",
                "data/my-data-mod/recipe/a.json",
            ]
        );
    }

    #[test]
    fn fabric_jar_with_code_is_refused() {
        let dir = Scratch::new();
        let jar = write_jar(
            &dir.0,
            "coded.jar",
            &[
                ("fabric.mod.json", FABRIC),
                ("com/example/Mod.class", b"\xCA\xFE\xBA\xBE"),
            ],
        );
        assert!(detect(&jar).unwrap().has_code);
        let e = package(&jar).err().unwrap();
        assert!(e.contains("JVM sidecar"), "{e}");
    }

    #[test]
    fn neoforge_jar_is_recognised_and_refused() {
        let dir = Scratch::new();
        let jar = write_jar(
            &dir.0,
            "neo.jar",
            &[("META-INF/neoforge.mods.toml", NEOFORGE)],
        );
        let d = detect(&jar).unwrap();
        assert_eq!(d.loader, Loader::NeoForge);
        assert_eq!(d.id, "neo_thing");
        assert_eq!(d.name, "Neo Thing");
        let e = package(&jar).err().unwrap();
        assert_eq!(e, "NeoForge mods are not supported (only Fabric data mods)");
    }

    #[test]
    fn datapack_takes_its_file_stem_as_id() {
        let dir = Scratch::new();
        let zip = write_jar(
            &dir.0,
            "pack_one.zip",
            &[("pack.mcmeta", PACK), ("data/foo/recipe/b.json", RECIPE)],
        );
        assert_eq!(detect(&zip).unwrap().loader, Loader::DataPack);
        let pkg = package(&zip).unwrap();
        assert_eq!(pkg.manifest.id, "pack_one");
        assert_eq!(pkg.manifest.namespaces, ["foo"]);
    }

    #[test]
    fn entries_that_leave_the_jar_are_ignored() {
        let dir = Scratch::new();
        let jar = write_jar(
            &dir.0,
            "tricky.jar",
            &[
                ("fabric.mod.json", FABRIC),
                ("../evil/Bad.class", b"x"),
                ("/abs/Bad.class", b"x"),
                ("data/my-data-mod/recipe/a.json", RECIPE),
            ],
        );
        let d = detect(&jar).unwrap();
        assert!(!d.has_code);
        assert!(!d.namespaces.iter().any(|ns| ns == "evil"));
        assert!(package(&jar).is_ok());
    }

    #[test]
    fn class_hidden_under_data_counts_as_code() {
        let dir = Scratch::new();
        let jar = write_jar(
            &dir.0,
            "hidden.jar",
            &[
                ("fabric.mod.json", FABRIC),
                ("data/my-data-mod/Mod.class", b"x"),
            ],
        );
        assert!(detect(&jar).unwrap().has_code);
        assert!(package(&jar).is_err());
    }

    #[test]
    fn game_namespace_is_not_a_mod_id() {
        let dir = Scratch::new();
        let zip = write_jar(
            &dir.0,
            "minecraft.zip",
            &[
                ("pack.mcmeta", PACK),
                ("data/minecraft/recipe/b.json", RECIPE),
            ],
        );
        assert!(package(&zip).is_err());
    }

    #[test]
    fn a_file_that_is_not_a_zip_is_an_error() {
        let dir = Scratch::new();
        let junk = dir.0.join("junk.jar");
        std::fs::write(&junk, b"not a zip").unwrap();
        assert!(detect(&junk).is_err());
        assert!(package(&junk).is_err());
    }
}
