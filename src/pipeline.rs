use std::{borrow::Cow, io::Write, path::PathBuf};

use crate::{
    image, BaseItemTypes, Bundle, BundleFs, DatString, Image, ImageError, IndexBundle,
    ItemVisualIdentity, UniqueStashLayout, Words,
};

type CowStr = Cow<'static, str>;
type DynRenamer = dyn Fn(&File) -> Option<CowStr>;

pub struct Pipeline<F: BundleFs> {
    fs: F,
    out: PathBuf,
    selectors: Vec<Box<dyn Matcher>>,
    postprocess: Vec<(Box<dyn Matcher>, Box<dyn Postprocess>)>,
    rename: Vec<Box<DynRenamer>>,
}

impl<F: BundleFs> Pipeline<F> {
    pub fn new(fs: F, out: impl Into<PathBuf>) -> Self {
        Self {
            fs,
            out: out.into(),
            selectors: Vec::new(),
            postprocess: Vec::new(),
            rename: Vec::new(),
        }
    }

    pub fn select(&mut self, matcher: impl Matcher + 'static) -> &mut Self {
        self.selectors.push(Box::new(matcher));
        self
    }

    pub fn rename<T>(&mut self, renamer: impl Fn(&File) -> Option<T> + 'static) -> &mut Self
    where
        T: Into<CowStr>,
    {
        self.rename
            .push(Box::new(move |file| renamer(file).map(Into::into)));
        self
    }

    pub fn postprocess(
        &mut self,
        matcher: impl Matcher + 'static,
        postprocess: impl Postprocess + 'static,
    ) -> &mut Self {
        self.postprocess
            .push((Box::new(matcher), Box::new(postprocess)));
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
            id: AnyString::DatString(base.id),
            item_visual_identity: base.item_visual_identity,
            name: AnyString::DatString(base.name),
        });

        let uniques = uniques.iter().map(|unique| {
            // TODO: this is trash, vis gets quereid later again, no error handling
            let name = words
                .get(unique.words as usize)
                .expect("word for unique")
                .text;
            let id = vis
                .get(unique.item_visual_identity as usize)
                .expect("vis for unique")
                .id;

            File {
                kind: Kind::Unique,
                id: AnyString::DatString(id),
                item_visual_identity: unique.item_visual_identity,
                name: AnyString::DatString(name),
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

            if vis.is_alternate_art {
                // Alternate art shares the name with non alternate art and would override it.
                continue;
            }

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

            let Ok(mut dds) = image::Dds::try_from(&*dds) else {
                tracing::warn!("unable to read dds {dds_file}");
                continue;
            };

            for (m, pp) in &self.postprocess {
                if m.matches(&item) {
                    pp.postprocess(&mut dds)?;
                }
            }

            let mut name = Cow::Owned(name);
            for rename in &self.rename {
                if let Some(new_name) = rename(&item) {
                    name = new_name
                }
            }

            self.write_image(&name, &dds)?;

            tracing::debug!("generated file '{name}'");
            total += 1;
        }

        for file in self.ui_images(&index)? {
            let Kind::Art { art_file, position, size } = file.kind else {
                unreachable!("ui images generated non art kind");
            };

            let Some(dds) = index.read_by_name(&art_file)? else {
                tracing::warn!("dds file '{art_file}' does not exist");
                continue;
            };

            let Ok(mut dds) = image::Dds::try_from(&*dds) else {
                tracing::warn!("unable to read dds {art_file}");
                continue;
            };

            dds.crop(position, size)?;

            let name = String::try_from(file.name)?;
            self.write_image(&name, &dds)?;

            tracing::debug!("generated art file '{name}'");
            total += 1;
        }

        tracing::info!("extracted a total of {total} assets");

        Ok(())
    }

    fn write_image(&self, name: &str, dds: &image::Dds) -> anyhow::Result<()> {
        let out = self.out.join(format!("{name}.webp"));

        std::fs::create_dir_all(out.parent().unwrap())?;
        {
            let mut out = std::fs::File::create(&out)?;
            out.write_all(&dds.write_blob("webp")?)?;
        }
        Ok(())
    }

    fn ui_images<F2: BundleFs>(
        &self,
        index: &IndexBundle<F2>,
    ) -> anyhow::Result<impl Iterator<Item = File<'static>> + '_> {
        let Some(ui_images) = index.read_by_name("Art/UIImages1.txt")? else {
            anyhow::bail!("Art/UIImages1.txt does not exist");
        };

        // TODO: this sucks but w/e, need something now
        let ui_images: String = (&DatString(&ui_images)).try_into()?;

        Ok(ui_images
            .lines()
            .filter_map(|line| {
                let (l, r) = line.split_once("\" \"")?;
                let name = l.strip_prefix('"')?;
                let (file, args) = r.split_once("\" ")?;

                let mut args = args
                    .split_whitespace()
                    .map(|x| x.parse::<u32>().expect("parse number"));
                let p1 = (args.next()?, args.next()?);
                let p2 = (args.next()?, args.next()?);

                let f = File {
                    kind: Kind::Art {
                        art_file: file.to_owned(),
                        position: p1,
                        size: (p2.0 - p1.0, p2.1 - p1.1),
                    },
                    name: AnyString::String(Cow::Borrowed(name)),
                    id: AnyString::String(Cow::Borrowed(name)),
                    item_visual_identity: 0,
                };

                if self.selectors.iter().any(|s| s.matches(&f)) {
                    Some(File {
                        kind: f.kind,
                        name: AnyString::String(Cow::Owned(name.to_owned())),
                        id: AnyString::String(Cow::Owned(name.to_owned())),
                        item_visual_identity: 0,
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .into_iter())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Art {
        art_file: String,
        position: (u32, u32),
        size: (u32, u32),
    },
    Base,
    Unique,
}

#[derive(Debug)]
pub enum AnyString<'a> {
    String(Cow<'a, str>),
    DatString(DatString<'a>),
}

impl<'a> AnyString<'a> {
    pub fn ends_with(&self, suffix: &str) -> bool {
        match self {
            Self::String(s) => s.ends_with(suffix),
            Self::DatString(s) => s.ends_with(suffix),
        }
    }

    pub fn starts_with(&self, prefix: &str) -> bool {
        match self {
            Self::String(s) => s.starts_with(prefix),
            Self::DatString(s) => s.starts_with(prefix),
        }
    }
}

impl<'a> TryFrom<&AnyString<'a>> for String {
    type Error = std::char::DecodeUtf16Error;

    fn try_from(s: &AnyString<'a>) -> Result<Self, Self::Error> {
        match s {
            AnyString::String(s) => Ok(s.to_string()),
            AnyString::DatString(s) => s.try_into(),
        }
    }
}

impl<'a> TryFrom<AnyString<'a>> for String {
    type Error = std::char::DecodeUtf16Error;

    fn try_from(s: AnyString<'a>) -> Result<Self, Self::Error> {
        match s {
            AnyString::String(s) => Ok(s.into_owned()),
            AnyString::DatString(s) => (&s).try_into(),
        }
    }
}

#[derive(Debug)]
pub struct File<'a> {
    pub kind: Kind,
    pub id: AnyString<'a>,
    pub name: AnyString<'a>,
    pub item_visual_identity: u64,
}

pub trait Matcher {
    fn matches(&self, item: &File) -> bool;
}

impl<F: Fn(&File) -> bool> Matcher for F {
    fn matches(&self, item: &File) -> bool {
        self(item)
    }
}

pub trait Postprocess {
    fn postprocess(&self, image: &mut Image) -> Result<(), ImageError>;
}

impl<F: Fn(&mut Image) -> Result<(), ImageError>> Postprocess for F {
    fn postprocess(&self, image: &mut Image) -> Result<(), ImageError> {
        self(image)
    }
}
