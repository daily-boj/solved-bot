use anyhow::anyhow;
use once_cell::sync::Lazy;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_repr::Deserialize_repr;
use std::fmt::{self, Display};

pub enum Tier {
    Bronze,
    Silver,
    Gold,
    Platinum,
    Diamond,
    Ruby,
}

impl Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Bronze => "Bronze",
                Self::Silver => "Silver",
                Self::Gold => "Gold",
                Self::Platinum => "Platinum",
                Self::Diamond => "Diamond",
                Self::Ruby => "Ruby",
            }
        )
    }
}

#[derive(Deserialize_repr)]
#[repr(u8)]
pub enum ClassDecoration {
    Normal = 0,
    Silver = 1,
    Gold = 2,
}

pub enum Level {
    Unranked,
    Ranked(Tier, u8),
}

impl<'de> Deserialize<'de> for Level {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let level = u8::deserialize(deserializer)?;
        match level {
            0 => Ok(Level::Unranked),
            _ => {
                let tier = match level {
                    1..=5 => Tier::Bronze,
                    6..=10 => Tier::Silver,
                    11..=15 => Tier::Gold,
                    16..=20 => Tier::Platinum,
                    21..=25 => Tier::Diamond,
                    26..=30 => Tier::Ruby,
                    _ => {
                        return Err(serde::de::Error::custom(format!(
                            "level out of range: {}",
                            level
                        )))
                    }
                };
                Ok(Level::Ranked(tier, 5 - (level - 1) % 5))
            }
        }
    }
}

impl Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unranked => write!(f, "Unranked"),
            Self::Ranked(tier, grade) => write!(
                f,
                "{} {}",
                tier,
                match grade {
                    1 => "I",
                    2 => "II",
                    3 => "III",
                    4 => "IV",
                    5 => "V",
                    _ => unreachable!(),
                }
            ),
        }
    }
}

#[derive(Deserialize)]
pub struct Search {
    pub autocomplete: Vec<AutoComplete>,
    pub problems: Vec<Problem>,
    #[serde(deserialize_with = "deserialize_u32_or_empty_list")]
    pub problem_count: u32,
    pub users: Vec<User>,
    #[serde(deserialize_with = "deserialize_u32_or_empty_list")]
    pub user_count: u32,
    pub algorithms: Vec<Algorithm>,
    #[serde(deserialize_with = "deserialize_u32_or_empty_list")]
    pub algorithm_count: u32,
    pub wiki_articles: Vec<Wiki>,
}

fn deserialize_u32_or_empty_list<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    struct CustomVisitor;
    impl<'de> serde::de::Visitor<'de> for CustomVisitor {
        type Value = u32;
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an integer or an empty list")
        }
        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v as u32)
        }
        fn visit_seq<A>(self, _v: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            Ok(0)
        }
    }
    deserializer.deserialize_any(CustomVisitor)
}

#[derive(Deserialize)]
pub struct AutoComplete {
    pub caption: String,
    pub description: String,
}

#[derive(Deserialize)]
pub struct Problem {
    pub id: u32,
    pub title: String,
    pub level: Level,
    pub solved: usize,
    pub caption: String,
    pub description: String,
    pub href: String,
}

#[derive(Deserialize)]
pub struct User {
    pub user_id: String,
    pub bio: String,
    pub profile_image_url: Option<String>,
    pub solved: u32,
    pub exp: u64,
    pub level: Level,
    pub class: u8,
    pub class_decoration: ClassDecoration,
    pub vote_count: u32,
    pub rank: u32,
}

#[derive(Deserialize)]
pub struct Algorithm {
    pub tag_name: String,
    pub full_name_en: String,
    pub full_name_ko: String,
    pub problem_count: u32,
    pub caption: String,
    pub description: String,
    pub href: String,
}

#[derive(Deserialize)]
pub struct Wiki {
    pub title: String,
    pub caption: String,
    pub description: String,
    pub href: String,
}

#[derive(Deserialize)]
struct ApiResult<T: DeserializeOwned> {
    #[serde(bound(deserialize = "T: DeserializeOwned"))]
    result: Option<T>,
}

pub async fn search(query: &str) -> anyhow::Result<Search> {
    #[derive(Serialize)]
    struct SearchQuery<'a> {
        query: &'a str,
    }

    static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| reqwest::Client::new());
    let result = CLIENT
        .get("https://api.solved.ac/v2/search/recommendations.json")
        .query(&[("query", query)])
        .send()
        .await?
        .json::<ApiResult<Search>>()
        .await?;

    result
        .result
        .ok_or_else(|| anyhow!("Unsuccessful solved.ac search: {}", query))
}
