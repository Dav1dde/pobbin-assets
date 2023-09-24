use bpaf::Bpaf;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
struct Args {
    #[bpaf(external, optional)]
    fs: Option<Fs>,

    #[bpaf(external, optional)]
    cache: Option<Cache>,

    #[bpaf(external)]
    action: Action,
}

#[derive(Debug, Clone, Bpaf)]
enum Fs {
    Patch {
        /// Patch version of the bundle for the PoE patch CDN.
        #[bpaf(argument("PATCH"))]
        patch: String,
    },
    Web {
        /// Base URL for the bundle.
        #[bpaf(argument("URL"))]
        web: String,
    },
    Local {
        /// Local path to bundle.
        #[bpaf(argument("PATH"))]
        path: String,
    },
}

#[derive(Debug, Clone, Bpaf)]
enum Cache {
    /// In memory filesystem cache.
    InMemoryCache,
    LocalCache {
        /// Local filesystem cache.
        #[bpaf(argument("PATH"))]
        local_cache: std::path::PathBuf,
    },
}

#[derive(Debug, Clone, Bpaf)]
enum Action {
    /// Print the SHA-256 hash of a bundled file.
    #[bpaf(command)]
    Sha(String),
    /// Extract a file to the current directory.
    #[bpaf(command)]
    Extract(String),
    /// Runs the asset pipeline.
    #[bpaf(command)]
    Assets {
        /// Output directory.
        #[bpaf(short('o'), argument("PATH"), fallback("./out".into()))]
        out: std::path::PathBuf,
    },
    /// Runs the data extraction pipeline.
    #[bpaf(command)]
    Data {
        /// Output directory.
        #[bpaf(short('o'), argument("PATH"), fallback("./out".into()))]
        out: std::path::PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let args = args().run();

    tracing_subscriber::fmt::init();

    let fs: Box<dyn pobbin_assets::BundleFs> = match args.fs {
        Some(Fs::Patch { patch }) => Box::new(pobbin_assets::WebBundleFs::cdn(&patch)),
        Some(Fs::Web { web }) => Box::new(pobbin_assets::WebBundleFs::new(web)),
        Some(Fs::Local { path }) => Box::new(pobbin_assets::LocalBundleFs::new(path)),
        None => Box::new(pobbin_assets::WebBundleFs::cdn(
            &pobbin_assets::latest_patch_version()?,
        )),
    };

    let fs: Box<dyn pobbin_assets::BundleFs> = match args.cache {
        Some(Cache::InMemoryCache) => Box::new(pobbin_assets::CacheBundleFs::new(
            fs,
            pobbin_assets::InMemoryCache::new(),
        )),
        Some(Cache::LocalCache { local_cache }) => Box::new(pobbin_assets::CacheBundleFs::new(
            fs,
            pobbin_assets::LocalCache::new(local_cache),
        )),
        None => fs,
    };

    match args.action {
        Action::Sha(file) => sha(fs, &file),
        Action::Extract(file) => extract(fs, &file),
        Action::Assets { out } => assets(fs, out),
        Action::Data { out } => data(fs, out),
    }
}

fn sha<F: pobbin_assets::BundleFs>(fs: F, file: &str) -> anyhow::Result<()> {
    let bundle = pobbin_assets::Bundle::new(fs);
    let index = bundle.index()?;

    let contents = index
        .read_by_name(file)?
        .ok_or_else(|| anyhow::anyhow!("file {file} can not be found"))?;

    let sha256 = {
        let mut hasher = Sha256::new();
        hasher.update(contents);
        hasher.finalize()
    };
    println!("{sha256:x}");

    Ok(())
}

fn extract<F: pobbin_assets::BundleFs>(fs: F, file: &str) -> anyhow::Result<()> {
    let bundle = pobbin_assets::Bundle::new(fs);
    let index = bundle.index()?;

    let contents = index
        .read_by_name(file)?
        .ok_or_else(|| anyhow::anyhow!("file {file} can not be found"))?;

    let path = std::path::PathBuf::from(file);
    std::fs::write(path.file_name().unwrap(), contents)?;

    Ok(())
}

fn assets<F: pobbin_assets::BundleFs>(fs: F, out: std::path::PathBuf) -> anyhow::Result<()> {
    use pobbin_assets::{File, Image, Kind};

    if !out.is_dir() {
        anyhow::bail!("out path '{}' is not a directory", out.display());
    }

    #[rustfmt::skip]
    pobbin_assets::Pipeline::new(fs, out)
        .font("Art/2DArt/Fonts/Fontin-SmallCaps.ttf")
        .select(|file: &File| file.id.starts_with("Metadata/Items/Gems"))
        .select(|file: &File| file.id.starts_with("Metadata/Items/Belts"))
        .select(|file: &File| file.id.starts_with("Metadata/Items/Rings"))
        .select(|file: &File| file.id.starts_with("Metadata/Items/Flasks"))
        .select(|file: &File| file.id.starts_with("Metadata/Items/Amulet"))
        .select(|file: &File| file.id.starts_with("Metadata/Items/Amulets"))
        .select(|file: &File| file.id.starts_with("Metadata/Items/Armours"))
        .select(|file: &File| file.id.starts_with("Metadata/Items/Jewels"))
        .select(|file: &File| file.id.starts_with("Metadata/Items/Quivers"))
        .select(|file: &File| file.id.starts_with("Metadata/Items/Weapons"))
        .select(|file: &File| file.id.starts_with("Metadata/Items/Trinkets"))
        .select(|file: &File| file.kind == Kind::Unique)
        .select(|file: &File| {
            file.id
                .starts_with("Art/2DArt/UIImages/InGame/AncestralTrial/PassiveTreeTattoos")
        })
        .select(|file: &File| file.id.starts_with("Art/2DArt/UIImages/InGame/ItemsHeader"))
        .select(|file: &File| file.id.starts_with("Art/2DArt/UIImages/Common/IconDex"))
        .select(|file: &File| file.id.starts_with("Art/2DArt/UIImages/Common/IconInt"))
        .select(|file: &File| file.id.starts_with("Art/2DArt/UIImages/Common/IconStr"))
        .select(|file: &File| {
            file.id.starts_with("art/2dart/skillicons/passives/")
                && file.id.ends_with("dds")
                && !file.id.contains("/4k/")
        })
        .select(|file: &File| {
            file.id
                .starts_with("art/2dart/skillicons/passives/masterypassiveicons/")
                && file.id.ends_with("dds")
        })
        .select(|file: &File| {
            file.id
                .starts_with("Art/2DArt/UIImages/InGame/ItemsSeparator")
        })
        .select(|file: &File| {
            file.id.starts_with("Art/2DArt/UIImages/InGame/") && file.id.ends_with("ItemSymbol")
        })
        .rename(|file| file.id.ends_with("BootsAtlas1").then_some("TwoTonedEvEs").map(Into::into))
        .rename(|file| file.id.ends_with("BootsAtlas2").then_some("TwoTonedArEv").map(Into::into))
        .rename(|file| file.id.ends_with("BootsAtlas3").then_some("TwoTonedArEs").map(Into::into))
        .rename(|file| file.name.contains('’').then(|| file.name.replace('’', "'")).map(Into::into))
        .rename(|file| file.id.starts_with("Metadata/Items/Gems").then_some(file.name.as_ref()).map(Into::into))
        .rename(|file| file.id.starts_with("Metadata/Items/Gems").then_some(file.id.as_ref()).map(Into::into))
        .postprocess(
            |file: &File| {
                file.id.starts_with("Metadata/Items/Flasks") || file.id.starts_with("UniqueFlask")
            },
            |image: &mut Image| image.flask(),
        )
        .execute()?;

    Ok(())
}

fn data<F: pobbin_assets::BundleFs>(fs: F, out: std::path::PathBuf) -> anyhow::Result<()> {
    let data = pobbin_assets::data::generate(fs)?;

    let gems = std::fs::File::create(out.join("gems.json"))?;
    serde_json::to_writer(gems, &data.gems)?;

    Ok(())
}
