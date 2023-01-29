use std::path::PathBuf;

use crate::{image, BaseItemTypes, Bundle, BundleFs, ItemVisualIdentity};

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

        let Some(base) = index.read::<BaseItemTypes>()? else {
            anyhow::bail!("BaseItemTypes table does not exist");
        };
        let Some(vis) = index.read::<ItemVisualIdentity>()? else {
            anyhow::bail!("ItemVisualIdentity table does not exist");
        };

        let bases = base
            .iter()
            .filter(|f| self.selectors.iter().any(|s| s.matches(f)))
            .map(|base| {
                let idx = base.item_visual_identity as usize;
                (base, vis.get(idx))
            });

        for (item, vis) in bases {
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
            dds.write_to_file(&out)?;

            tracing::info!("generated file '{name}' -> {}", out.display());
        }

        Ok(())
    }
}

pub type File<'a> = BaseItemTypes<'a>;

pub trait Matcher {
    fn matches(&self, item: &File) -> bool;
}

impl<F: Fn(&File) -> bool> Matcher for F {
    fn matches(&self, item: &File) -> bool {
        self(item)
    }
}

pub mod matchers {
    use super::*;
    use crate::DatString;

    pub struct Extractor<F>(F);

    impl<F> Extractor<F>
    where
        F: for<'a, 'b> Fn(&'a File<'b>) -> &'a DatString<'b>,
    {
        pub fn starts_with(
            self,
            prefix: &str,
        ) -> StringMatcher<F, impl Fn(&DatString) -> bool + '_> {
            StringMatcher {
                extract: self.0,
                matcher: move |s: &DatString| s.starts_with(prefix),
            }
        }
    }

    pub struct StringMatcher<E, M> {
        extract: E,
        matcher: M,
    }

    impl<E, M> StringMatcher<E, M> {}

    impl<E, M> Matcher for StringMatcher<E, M>
    where
        E: for<'a, 'b> Fn(&'a File<'b>) -> &'a DatString<'b>,
        M: Fn(&DatString) -> bool,
    {
        fn matches(&self, item: &File) -> bool {
            (self.matcher)((self.extract)(item))
        }
    }

    pub fn id() -> Extractor<impl for<'a, 'b> Fn(&'a File<'b>) -> &'a DatString<'b>> {
        fn extractor<'a, 'b>(f: &'a File<'b>) -> &'a DatString<'b> {
            &f.id
        }
        Extractor(extractor)
    }
}
