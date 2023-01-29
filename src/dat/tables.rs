use byteorder::{ReadBytesExt, LE};

use super::{DatString, Row, VarDataReader};

#[derive(Debug)]
pub struct BaseItemTypes<'a> {
    pub id: DatString<'a>,
    pub name: DatString<'a>,
    pub item_visual_identity: u64,
}

impl<'ty> Row for BaseItemTypes<'ty> {
    const FILE: &'static str = "Data/BaseItemTypes.dat64";
    const SIZE: usize = 279;

    type Item<'a> = BaseItemTypes<'a>;

    fn parse<'a>(data: &'a [u8], var_data: VarDataReader<'a>) -> Self::Item<'a> {
        let id = var_data.read_string((&data[0..]).read_u64::<LE>().unwrap() as usize);
        let name = var_data.read_string((&data[32..]).read_u64::<LE>().unwrap() as usize);
        let item_visual_identity = (&data[128..]).read_u64::<LE>().unwrap();

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
}

impl<'ty> Row for ItemVisualIdentity<'ty> {
    const FILE: &'static str = "Data/ItemVisualIdentity.dat64";
    const SIZE: usize = 533;

    type Item<'a> = ItemVisualIdentity<'a>;

    fn parse<'a>(data: &'a [u8], var_data: VarDataReader<'a>) -> ItemVisualIdentity<'a> {
        let id = var_data.read_string((&data[0..]).read_u64::<LE>().unwrap() as usize);
        let dds_file = var_data.read_string((&data[8..]).read_u64::<LE>().unwrap() as usize);

        ItemVisualIdentity { id, dds_file }
    }
}
