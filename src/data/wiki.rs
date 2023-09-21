use std::marker::PhantomData;

use itertools::Itertools;
use serde::{
    de::{self, DeserializeOwned},
    Deserialize,
};

const WIKI_BASE: &str = "https://www.poewiki.net/";

pub fn cargo_fetch<T>(
    query: &[(&str, &str)],
) -> anyhow::Result<impl Iterator<Item = anyhow::Result<T>>>
where
    T: DeserializeOwned,
{
    let mut url = url::Url::parse(WIKI_BASE)?.join("/w/api.php")?;

    url.query_pairs_mut()
        .extend_pairs(query)
        .append_pair("action", "cargoquery")
        .append_pair("format", "json")
        .append_pair("limit", "500");

    Ok(CargoIter {
        url,
        limit: 500,
        offset: 0,
        page: PhantomData,
        is_done: false,
    }
    .flatten_ok())
}

struct CargoIter<T> {
    url: url::Url,
    limit: usize,
    offset: usize,
    page: PhantomData<T>,
    is_done: bool,
}

impl<T> Iterator for CargoIter<T>
where
    T: DeserializeOwned,
{
    type Item = anyhow::Result<Vec<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_done {
            return None;
        }

        let mut url = self.url.clone();
        url.query_pairs_mut()
            .append_pair("offset", &self.offset.to_string());

        let response = match ureq::request_url("GET", &url).call() {
            Ok(response) => response,
            Err(err) => return Some(Err(err.into())),
        };

        let reader = response.into_reader();
        let response: CargoResponse<_> = match serde_json::from_reader(reader) {
            Ok(response) => response,
            Err(err) => return Some(Err(err.into())),
        };

        self.offset += self.limit;

        self.is_done = response.data.len() < self.limit;

        if response.data.is_empty() {
            return None;
        }
        Some(Ok(response.data))
    }
}

#[derive(Debug, Deserialize)]
struct CargoResponse<T: DeserializeOwned> {
    #[serde(rename = "cargoquery", deserialize_with = "de_cargo_items")]
    data: Vec<T>,
}

fn de_cargo_items<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Visitor<T>(PhantomData<T>);

    impl<'de, T> de::Visitor<'de> for Visitor<T>
    where
        T: Deserialize<'de>,
    {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "wiki cargo items")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            #[derive(Deserialize)]
            struct Inner<T> {
                #[serde(rename = "title")]
                inner: T,
            }

            let mut result: Vec<T> = Vec::with_capacity(seq.size_hint().unwrap_or(0));
            while let Some(next) = seq.next_element::<Inner<T>>()? {
                result.push(next.inner)
            }

            Ok(result)
        }
    }

    deserializer.deserialize_seq(Visitor(PhantomData))
}
