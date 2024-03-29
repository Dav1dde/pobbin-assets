use std::collections::{BTreeSet, HashMap};

use anyhow::Context;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::{formats::CommaSeparator, DefaultOnNull, DisplayFromStr, StringWithSeparator};

use crate::{BaseItemTypes, Bundle, BundleFs, SkillGems};

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct Gems(Vec<Gem>);

#[derive(Debug, Serialize)]
pub struct Gem {
    /// Id of the gem.
    pub id: String,
    /// Name of the gem.
    pub name: String,
    /// Mininum level for the level 1 gem.
    pub level: u32,
    /// Color of the gem, one of `red`, `green`, `blue`, `white`.
    pub color: &'static str,
    /// Vendors selling the gem and after which quest they unlock.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub vendors: Vec<Vendor>,
}

#[derive(Debug, Serialize)]
pub struct Vendor {
    /// Name of the quest which unlocks this vendor.
    pub quest: String,
    /// Act of the vendor.
    pub act: u8,
    /// Name of the vendor in this specific act.
    pub npc: String,
    /// List of classes this vendor unlocks for, `None` means all.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_ids: Option<BTreeSet<String>>,
}

pub fn generate<F: BundleFs>(fs: F) -> anyhow::Result<Gems> {
    let vendor_gem_rewards = fetch_vendor_gem_rewards()?;
    tracing::info!(
        "fetched vendor rewards for {} gems",
        vendor_gem_rewards.len()
    );

    let bundle = Bundle::new(fs);
    let index = bundle.index()?;

    macro_rules! read {
        ($name:ident, $type:ty) => {
            let Some($name) = index.read::<$type>()? else {
                anyhow::bail!("{} table does not exist", stringify!($type));
            };
        };
    }

    read!(bits, BaseItemTypes);
    read!(skill_gems, SkillGems);

    let mut gems = Vec::with_capacity(skill_gems.len());
    for sg in skill_gems.iter() {
        let bit = bits
            .get(sg.base_item_type as usize)
            .with_context(|| format!("missing base item type {} for gem", sg.base_item_type))?;

        if bit.site_visibility == 0 {
            continue;
        }

        let id = String::try_from(&bit.id)?;
        let name = String::try_from(&bit.name)?;

        let mut vendors = vendor_gem_rewards
            .get(&id)
            .unwrap_or(&Vec::new())
            .iter()
            .map(|q| Vendor {
                quest: q.quest.clone(),
                act: q.act,
                class_ids: q.class_ids.clone(),
                npc: q.npc.clone(),
            })
            .collect::<Vec<_>>();

        vendors.sort_unstable_by(|a, b| (a.act, &b.quest).cmp(&(b.act, &b.quest)));

        gems.push(Gem {
            id,
            name,
            level: bit.drop_level,
            color: sg.color.as_str(),
            vendors,
        })
    }
    gems.sort_unstable_by(|a, b| a.id.cmp(&b.id));

    Ok(Gems(gems))
}

fn fetch_vendor_gem_rewards() -> anyhow::Result<HashMap<String, Vec<VendorGemReward>>> {
    let vendor_gem_rewards = super::wiki::cargo_fetch(&[
        ("tables", "items,vendor_rewards"),
        ("join_on", "items._pageID=vendor_rewards._pageID"),
        ("fields", "items.metadata_id,items.name,vendor_rewards.quest,vendor_rewards.act,vendor_rewards.class_ids,vendor_rewards.npc"),
        ("where", "vendor_rewards._pageID IS NOT null"),
    ])?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .into_group_map_by(|v: &VendorGemReward| v.id.clone());

    Ok(vendor_gem_rewards)
}

#[serde_with::serde_as]
#[derive(Debug, Deserialize)]
struct VendorGemReward {
    #[serde(rename = "metadata id")]
    #[serde_as(as = "DefaultOnNull")]
    id: String,
    quest: String,
    #[serde_as(as = "DisplayFromStr")]
    act: u8,
    #[serde(rename = "class ids")]
    #[serde_as(as = "Option<StringWithSeparator::<CommaSeparator, String>>")]
    class_ids: Option<BTreeSet<String>>,
    npc: String,
}
