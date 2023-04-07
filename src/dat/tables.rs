use super::{utils::parse_u64, DatString, Row, VarDataReader};

#[derive(Debug)]
pub struct BaseItemTypes<'a> {
    pub id: DatString<'a>,
    pub name: DatString<'a>,
    pub item_visual_identity: u64,
}

impl<'ty> Row for BaseItemTypes<'ty> {
    const FILE: &'static str = "Data/BaseItemTypes.dat64";
    const SIZE: usize = 296;

    type Item<'a> = BaseItemTypes<'a>;

    fn parse<'a>(data: &'a [u8], var_data: VarDataReader<'a>) -> Self::Item<'a> {
        let id = var_data.get_string_from(&data[0..]);
        let name = var_data.get_string_from(&data[32..]);
        let item_visual_identity = parse_u64(&data[128..]);

        BaseItemTypes {
            id,
            name,
            item_visual_identity,
        }
    }
}

#[derive(Debug)]
pub struct ItemVisualIdentity<'a> {
    pub id: DatString<'a>,
    pub dds_file: DatString<'a>,
    pub is_alternate_art: bool,
}

impl<'ty> Row for ItemVisualIdentity<'ty> {
    const FILE: &'static str = "Data/ItemVisualIdentity.dat64";
    const SIZE: usize = 533;

    type Item<'a> = ItemVisualIdentity<'a>;

    fn parse<'a>(data: &'a [u8], var_data: VarDataReader<'a>) -> ItemVisualIdentity<'a> {
        let id = var_data.get_string_from(&data[0..]);
        let dds_file = var_data.get_string_from(&data[8..]);
        let is_alternate_art = data[300] == 1;

        ItemVisualIdentity {
            id,
            dds_file,
            is_alternate_art,
        }
    }
}

#[derive(Debug)]
pub struct UniqueStashLayout {
    pub words: u64,
    pub item_visual_identity: u64,
}

impl Row for UniqueStashLayout {
    const FILE: &'static str = "Data/UniqueStashLayout.dat64";
    const SIZE: usize = 83;

    type Item<'a> = UniqueStashLayout;

    fn parse<'a>(data: &'a [u8], _var_data: VarDataReader<'a>) -> Self::Item<'a> {
        let words = parse_u64(&data[0..]);
        let item_visual_identity = parse_u64(&data[16..]);

        UniqueStashLayout {
            words,
            item_visual_identity,
        }
    }
}

#[derive(Debug)]
pub struct Words<'a> {
    pub text: DatString<'a>,
}

impl<'ty> Row for Words<'ty> {
    const FILE: &'static str = "Data/Words.dat64";
    const SIZE: usize = 64;

    type Item<'a> = Words<'a>;

    fn parse<'a>(data: &'a [u8], var_data: VarDataReader<'a>) -> Self::Item<'a> {
        let text = var_data.get_string_from(&data[4..]);

        Words { text }
    }
}
