use quote::ToTokens;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

type DeferredLink = LazyLock<Mutex<HashMap<UniqueItemId, Vec<(UniqueItemId, String)>>>>;
// ── Unique key: trait/enum name + number of generic params ────────────────────
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct UniqueItemId
{
    pub item_name:    String,
    pub num_generics: usize,
}

impl UniqueItemId
{
    pub fn new(item_name: String, num_generics: usize) -> Self { Self { item_name, num_generics } }
}

// ── Static registries ─────────────────────────────────────────────────────────
// Everything is serialised to String because syn types are !Send + !Sync.

static TRAIT_DEFS: LazyLock<Mutex<HashMap<UniqueItemId, String>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

static ENUM_DEFS: LazyLock<Mutex<HashMap<UniqueItemId, String>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

// key   = UniqueItemId of the thing NOT YET SEEN
// value = list of (waiting enum UniqueItemId, original trait path string)
static DEFERRED_LINKS: DeferredLink = LazyLock::new(|| Mutex::new(HashMap::new()));

// ── Trait cache ───────────────────────────────────────────────────────────────
pub fn cache_trait(item: &syn::ItemTrait)
{
    let uid = UniqueItemId::new(item.ident.to_string(), item.generics.params.len());
    TRAIT_DEFS.lock().unwrap().insert(uid, item.to_token_stream().to_string());
}

pub fn get_trait(name: &str, num_generics: usize) -> Option<syn::ItemTrait>
{
    let uid = UniqueItemId::new(name.to_string(), num_generics);
    TRAIT_DEFS.lock().unwrap().get(&uid).map(|s| syn::parse_str(s).unwrap())
}

// ── Enum cache ────────────────────────────────────────────────────────────────
pub fn cache_enum(item: &syn::ItemEnum)
{
    let uid = UniqueItemId::new(item.ident.to_string(), item.generics.params.len());
    ENUM_DEFS.lock().unwrap().insert(uid, item.to_token_stream().to_string());
}

pub fn get_enum(name: &str, num_generics: usize) -> Option<syn::ItemEnum>
{
    let uid = UniqueItemId::new(name.to_string(), num_generics);
    ENUM_DEFS.lock().unwrap().get(&uid).map(|s| syn::parse_str(s).unwrap())
}

// ── Deferred links ────────────────────────────────────────────────────────────
/// `waiting` will be resolved once `needed` is cached.
/// `trait_path_str` is the original trait path tokens (e.g. "Processor < T , T >")
/// so it can be reconstructed exactly when the deferred impl is generated.
pub fn defer_link(needed: UniqueItemId, waiting: UniqueItemId, trait_path_str: String) { DEFERRED_LINKS.lock().unwrap().entry(needed).or_default().push((waiting, trait_path_str)); }

/// Called after a trait is cached.
/// Returns every enum that was already registered and waiting for this trait,
/// paired with the original trait path string that was supplied by the user.
pub fn enums_waiting_for_trait(trait_name: &str, num_generics: usize) -> Vec<(syn::ItemEnum, syn::Path)>
{
    let uid = UniqueItemId::new(trait_name.to_string(), num_generics);
    let waiters = DEFERRED_LINKS.lock().unwrap().remove(&uid).unwrap_or_default();
    waiters
        .into_iter()
        .filter_map(|(id, path_str)| {
            let enum_item = get_enum(&id.item_name, id.num_generics)?;
            let path: syn::Path = syn::parse_str(&path_str).ok()?;
            Some((enum_item, path))
        })
        .collect()
}

/// Called after an enum is cached.
/// Returns every trait that was already registered and waiting for this enum.
pub fn traits_waiting_for_enum(enum_name: &str, num_generics: usize) -> Vec<syn::ItemTrait>
{
    let uid = UniqueItemId::new(enum_name.to_string(), num_generics);
    let waiters = DEFERRED_LINKS.lock().unwrap().remove(&uid).unwrap_or_default();
    waiters.into_iter().filter_map(|(id, _path_str)| get_trait(&id.item_name, id.num_generics)).collect()
}
