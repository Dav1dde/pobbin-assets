use super::{row::ParseError, utils::parse_u64, DatString, Row, VarDataReader};
use crate::dat::utils::parse_u32;

#[derive(Debug)]
pub struct BaseItemTypes<'a> {
    pub id: DatString<'a>,
    pub name: DatString<'a>,
    pub drop_level: u32,
    pub site_visibility: u32,
    pub item_visual_identity: u64,
}

impl<'ty> Row for BaseItemTypes<'ty> {
    const FILE: &'static str = "Data/BaseItemTypes.datc64";

    type Item<'a> = BaseItemTypes<'a>;

    fn parse<'a>(
        data: &'a [u8],
        var_data: VarDataReader<'a>,
    ) -> Result<Self::Item<'a>, ParseError> {
        let id = var_data.get_string_from(data, 0)?;
        let name = var_data.get_string_from(data, 32)?;
        let drop_level = parse_u32(data, 48)?;
        let site_visibility = parse_u32(data, 124)?;
        let item_visual_identity = parse_u64(data, 128)?;

        Ok(BaseItemTypes {
            id,
            name,
            drop_level,
            site_visibility,
            item_visual_identity,
        })
    }
}

#[derive(Debug)]
pub struct ItemVisualIdentity<'a> {
    pub id: DatString<'a>,
    pub dds_file: DatString<'a>,
    pub is_alternate_art: bool,
}

impl<'ty> Row for ItemVisualIdentity<'ty> {
    const FILE: &'static str = "Data/ItemVisualIdentity.datc64";

    type Item<'a> = ItemVisualIdentity<'a>;

    fn parse<'a>(
        data: &'a [u8],
        var_data: VarDataReader<'a>,
    ) -> Result<Self::Item<'a>, ParseError> {
        let id = var_data.get_string_from(data, 0)?;
        let dds_file = var_data.get_string_from(data, 8)?;
        let is_alternate_art = data[300] == 1;

        Ok(ItemVisualIdentity {
            id,
            dds_file,
            is_alternate_art,
        })
    }
}

#[derive(Debug)]
pub struct UniqueStashLayout {
    pub words: u64,
    pub item_visual_identity: u64,
    pub show_if_empty_challenge_league: bool,
}

impl Row for UniqueStashLayout {
    const FILE: &'static str = "Data/UniqueStashLayout.datc64";

    type Item<'a> = UniqueStashLayout;

    fn parse<'a>(
        data: &'a [u8],
        _var_data: VarDataReader<'a>,
    ) -> Result<Self::Item<'a>, ParseError> {
        let words = parse_u64(data, 0)?;
        let item_visual_identity = parse_u64(data, 16)?;
        let show_if_empty_challenge_league = data[64] == 1;

        Ok(UniqueStashLayout {
            words,
            item_visual_identity,
            show_if_empty_challenge_league,
        })
    }
}

#[derive(Debug)]
pub struct Words<'a> {
    pub text2: DatString<'a>,
}

impl<'ty> Row for Words<'ty> {
    const FILE: &'static str = "Data/Words.datc64";

    type Item<'a> = Words<'a>;

    fn parse<'a>(
        data: &'a [u8],
        var_data: VarDataReader<'a>,
    ) -> Result<Self::Item<'a>, ParseError> {
        let text2 = var_data.get_string_from(data, 48)?;

        Ok(Words { text2 })
    }
}

#[derive(Debug)]
pub struct SkillGems {
    pub base_item_type: u64,
    pub str: u32,
    pub dex: u32,
    pub int: u32,
    pub color: Color,
}

#[derive(Debug)]
pub enum Color {
    Red,
    Green,
    Blue,
    White,
}

impl Color {
    pub fn as_str(&self) -> &'static str {
        match self {
            Color::Red => "red",
            Color::Green => "green",
            Color::Blue => "blue",
            Color::White => "white",
        }
    }
}

impl Row for SkillGems {
    const FILE: &'static str = "Data/SkillGems.datc64";

    type Item<'a> = SkillGems;

    fn parse<'a>(
        data: &'a [u8],
        _var_data: VarDataReader<'a>,
    ) -> Result<Self::Item<'a>, ParseError> {
        let base_item_type = parse_u64(data, 0)?;
        let str = parse_u32(data, 32)?;
        let dex = parse_u32(data, 36)?;
        let int = parse_u32(data, 40)?;
        let color = match parse_u32(data, 83)? {
            1 => Color::Red,
            2 => Color::Green,
            3 => Color::Blue,
            4 => Color::White,
            _ => return Err(ParseError::InvalidData),
        };

        Ok(SkillGems {
            base_item_type,
            str,
            dex,
            int,
            color,
        })
    }
}
