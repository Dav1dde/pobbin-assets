use std::{io::Write, path::PathBuf};

use crate::{
    image, BaseItemTypes, Bundle, BundleFs, DatString, ItemVisualIdentity, UniqueStashLayout, Words,
};

pub struct Pipeline<F: BundleFs> {
    fs: F,
    out: PathBuf,
    selectors: Vec<Box<dyn Matcher>>,
}

impl<F: BundleFs> Pipeline<F> {
    pub fn new(fs: F, out: impl Into<PathBuf>) -> Self {
        Self {
            fs,
            out: out.into(),
            selectors: Vec::new(),
        }
    }

    pub fn select(&mut self, matcher: impl Matcher + 'static) -> &mut Self {
        self.selectors.push(Box::new(matcher));
        self
    }

    pub fn execute(&self) -> anyhow::Result<()> {
        let bundle = Bundle::new(&self.fs);
        let index = bundle.index()?;

        macro_rules! read {
            ($name:ident, $type:ty) => {
                let Some($name) = index.read::<$type>()? else {
                                    anyhow::bail!("{} table does not exist", stringify!($type));
                                };
            };
        }

        read!(bases, BaseItemTypes);
        read!(uniques, UniqueStashLayout);
        read!(words, Words);
        read!(vis, ItemVisualIdentity);

        let bases = bases.iter().map(|base| File {
            kind: Kind::Base,
            id: base.id,
            item_visual_identity: base.item_visual_identity,
            name: base.name,
        });

        let uniques = uniques.iter().map(|unique| {
            let name = words
                .get(unique.words as usize)
                .expect("word for unique")
                .text;
            File {
                kind: Kind::Unique,
                id: name,
                item_visual_identity: unique.item_visual_identity,
                name,
            }
        });

        let files = bases
            .chain(uniques)
            .filter(|f| self.selectors.iter().any(|s| s.matches(f)))
            .map(|base| {
                let idx = base.item_visual_identity as usize;
                (base, vis.get(idx))
            });

        let mut total = 0usize;
        for (item, vis) in files {
            let Some(vis) = vis else {
                tracing::warn!("item '{item:?}' has no visual identity");
                continue;
            };

            let Ok(name) = String::try_from(&item.name) else {
                tracing::warn!("invalid name on item '{item:?}'");
                continue;
            };
            let Ok(dds_file) = String::try_from(&vis.dds_file) else {
                tracing::warn!("invalid dds_file on item '{item:?}' and vis '{vis:?}'");
                continue;
            };

            let Some(dds) = index.read_by_name(&dds_file)? else {
                tracing::warn!("dds file '{dds_file}' does not exist");
                continue;
            };

            let Ok(dds) = image::Dds::try_from(&*dds) else {
                tracing::warn!("unable to read dds {dds_file}");
                continue;
            };

            let out = self.out.join(format!("{name}.webp"));
            {
                let mut out = std::fs::File::create(&out)?;
                out.write_all(&dds.write_blob("webp")?)?;
            }

            tracing::debug!("generated file '{name}' -> {}", out.display());
            total += 1;
        }

        tracing::info!("extracted a total of {total} assets");

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Base,
    Unique,
}

#[derive(Debug)]
pub struct File<'a> {
    pub kind: Kind,
    pub id: DatString<'a>,
    pub item_visual_identity: u64,
    pub name: DatString<'a>,
}

pub trait Matcher {
    fn matches(&self, item: &File) -> bool;
}

impl<F: Fn(&File) -> bool> Matcher for F {
    fn matches(&self, item: &File) -> bool {
        self(item)
    }
}
