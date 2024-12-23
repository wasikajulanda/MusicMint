#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct NFT {
    id: u64,
    title: String,
    artist: String,
    album: String,
    metadata_url: String,
    price: u64,
    created_at: u64,
    updated_at: Option<u64>,
}

impl Storable for NFT {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for NFT {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static STORAGE: RefCell<StableBTreeMap<u64, NFT, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct NFTPayload {
    title: String,
    artist: String,
    album: String,
    metadata_url: String,
    price: u64,
}

#[ic_cdk::query]
fn get_nft(id: u64) -> Result<NFT, Error> {
    match _get_nft(&id) {
        Some(nft) => Ok(nft),
        None => Err(Error::NotFound {
            msg: format!("An NFT with id={} not found", id),
        }),
    }
}

#[ic_cdk::update]
fn mint_nft(payload: NFTPayload) -> NFT {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment id counter");
    let nft = NFT {
        id,
        title: payload.title,
        artist: payload.artist,
        album: payload.album,
        metadata_url: payload.metadata_url,
        price: payload.price,
        created_at: time(),
        updated_at: None,
    };
    do_insert(&nft);
    nft
}

#[ic_cdk::update]
fn update_nft(id: u64, payload: NFTPayload) -> Result<NFT, Error> {
    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut nft) => {
            nft.title = payload.title;
            nft.artist = payload.artist;
            nft.album = payload.album;
            nft.metadata_url = payload.metadata_url;
            nft.price = payload.price;
            nft.updated_at = Some(time());
            do_insert(&nft);
            Ok(nft)
        }
        None => Err(Error::NotFound {
            msg: format!("Couldn't update an NFT with id={}. NFT not found", id),
        }),
    }
}

fn do_insert(nft: &NFT) {
    STORAGE.with(|service| service.borrow_mut().insert(nft.id, nft.clone()));
}

#[ic_cdk::update]
fn delete_nft(id: u64) -> Result<NFT, Error> {
    match STORAGE.with(|service| service.borrow_mut().remove(&id)) {
        Some(nft) => Ok(nft),
        None => Err(Error::NotFound {
            msg: format!("Couldn't delete an NFT with id={}. NFT not found.", id),
        }),
    }
}

#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
}

fn _get_nft(id: &u64) -> Option<NFT> {
    STORAGE.with(|service| service.borrow().get(id))
}

ic_cdk::export_candid!();
