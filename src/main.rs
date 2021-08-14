use rocksdb::{DB, ColumnFamilyDescriptor, Options};
use tokio::net::{TcpListener, TcpStream};
use tokio::join;
use std::{
    net::SocketAddr,
};
use web3::futures::{future, StreamExt};
use substrate_subxt::{
    balances::{
        AccountData,
        Balances,
        BalancesEventTypeRegistry,
    },
    session::{
        Session,
        SessionEventTypeRegistry,
    },
    staking::{
        Staking,
        StakingEventTypeRegistry,
    },
    sudo::{
        Sudo,
        SudoEventTypeRegistry,
    },
    system::{
        System,
        SystemEventTypeRegistry,
    },
    ClientBuilder,
    EventSubscription,
    sp_runtime::traits::{
        AtLeast32Bit,
        MaybeSerialize,
        Member,
    },
    EventTypeRegistry,
    extrinsic::{
        DefaultExtra,
    },
    BasicSessionKeys,
    Runtime,
};
use std::fmt::Debug;

use sp_runtime::{
    generic::Header,
    traits::{
        BlakeTwo256,
        IdentifyAccount,
        Verify,
    },
    MultiSignature,
    OpaqueExtrinsic,
};

use sp_io::hashing::{blake2_128, keccak_256};

use codec::{
    Codec,
    Decode,
    Encode,
};

use proc_macro::*;

use serde::Serialize;
use bincode;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AcuityRuntime;

impl Staking for AcuityRuntime {}

impl Runtime for AcuityRuntime {
    type Signature = MultiSignature;
    type Extra = DefaultExtra<Self>;

    fn register_type_sizes(event_type_registry: &mut EventTypeRegistry<Self>) {
        event_type_registry.with_system();
        event_type_registry.with_balances();
        event_type_registry.with_session();
        event_type_registry.with_staking();
        event_type_registry.with_sudo();
        substrate_subxt::register_default_type_sizes(event_type_registry);
    }
}

impl System for AcuityRuntime {
    type Index = u32;
    type BlockNumber = u32;
    type Hash = sp_core::H256;
    type Hashing = BlakeTwo256;
    type AccountId = <<MultiSignature as Verify>::Signer as IdentifyAccount>::AccountId;
    type Address = sp_runtime::MultiAddress<Self::AccountId, u32>;
    type Header = Header<Self::BlockNumber, BlakeTwo256>;
    type Extrinsic = OpaqueExtrinsic;
    type AccountData = AccountData<<Self as Balances>::Balance>;
}

impl Balances for AcuityRuntime {
    type Balance = u128;
}

impl Session for AcuityRuntime {
    type ValidatorId = <Self as System>::AccountId;
    type Keys = BasicSessionKeys;
}

impl Sudo for AcuityRuntime {}

impl AtomicSwap for AcuityRuntime {
    type Balance = u128;
    type Moment = u64;
}



#[module]
pub trait AtomicSwap: System {
    type Balance: Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerialize
        + Debug
        + From<<Self as System>::BlockNumber>;
    type Moment: Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerialize
        + Debug
        + From<<Self as System>::BlockNumber>;
}

/// AddToOrder event.
#[derive(Debug, Decode, Eq, Event, PartialEq)]
pub struct AddToOrderEvent<T: AtomicSwap> {
    pub seller: <T as System>::AccountId,
    pub asset_id: AssetId,
    pub price: u128,
    pub foreign_address: ForeignAddress,
    pub value: T::Balance,
}

/// RemoveFromOrder event.
#[derive(Debug, Decode, Eq, Event, PartialEq)]
pub struct RemoveFromOrderEvent<T: AtomicSwap> {
    pub seller: <T as System>::AccountId,
    pub asset_id: AssetId,
    pub price: u128,
    pub foreign_address: ForeignAddress,
    pub value: T::Balance,
}

/// LockSell event.
#[derive(Debug, Decode, Eq, Event, PartialEq)]
pub struct LockSellEvent<T: AtomicSwap> {
    pub hashed_secret: [u8; 32],
    pub order_id: [u8; 16],
    pub value: T::Balance,
    pub timeout: T::Moment,
}

/// UnlockSell event.
#[derive(Debug, Decode, Eq, Event, PartialEq)]
pub struct UnlockSellEvent<T: AtomicSwap> {
    pub secret: [u8; 32],
    pub buyer: <T as System>::AccountId,
}

/// TimeoutSell event.
#[derive(Debug, Decode, Eq, PartialEq)]
pub struct TimeoutSellEvent {
    pub hashed_secret: [u8; 32],
}

/// LockBuy event.
#[derive(Debug, Decode, Eq, Event, PartialEq)]
pub struct LockBuyEvent<T: AtomicSwap> {
    pub hashed_secret: [u8; 32],
    pub asset_id: AssetId,
    pub order_id: OrderId,
    pub seller: <T as System>::AccountId,
    pub value: T::Balance,
    pub timeout: T::Moment,
}

/// UnlockBuy event.
#[derive(Debug, Decode, Eq, PartialEq)]
pub struct UnlockBuyEvent {
    pub hashed_secret: [u8; 32],
}

/// TimeoutBuy event.
#[derive(Debug, Decode, Eq, PartialEq)]
pub struct TimeoutBuyEvent {
    pub hashed_secret: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Default, Serialize)]
pub struct Order {
    pub seller: Vec<u8>,
    pub asset_id: AssetId,
    pub price: u128,
    pub foreign_address: ForeignAddress,
}

impl Order {
    pub fn get_order_id(&self) -> OrderId {
        let mut order_id = OrderId::default();
        order_id.0.copy_from_slice(&blake2_128(&[self.seller.encode(), self.asset_id.encode(), self.price.to_ne_bytes().to_vec(), self.foreign_address.encode()].concat()));
        order_id
    }
}

/// An Order Id (i.e. 16 bytes).
///
/// This gets serialized to the 0x-prefixed hex representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, Default)]
pub struct OrderId([u8; 16]);

impl AsRef<[u8]> for OrderId {
    fn as_ref(&self) -> &[u8] {
		&self.0
    }
}

/// An Asset Id (i.e. 16 bytes).
///
/// This gets serialized to the 0x-prefixed hex representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, Default, Serialize)]
pub struct AssetId([u8; 16]);

/// A Foreign Address (i.e. 32 bytes).
///
/// This gets serialized to the 0x-prefixed hex representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, Default, Serialize)]
pub struct ForeignAddress([u8; 32]);

#[tokio::main]
async fn main() {
    let mut db_opts = Options::default();
    db_opts.create_missing_column_families(true);
    let path = "database";
    let cf1 = ColumnFamilyDescriptor::new("order_static", Options::default());
    let cf2 = ColumnFamilyDescriptor::new("order_value", Options::default());
    let cf3 = ColumnFamilyDescriptor::new("order_list", Options::default());
    let db = DB::open_cf_descriptors(&db_opts, path, vec![cf1, cf2, cf3]).unwrap();
    // Spawn websockets task.
    let websockets_task = tokio::spawn(websockets_listen());
    // Spawn Acuity task.
    let acuity_task = tokio::spawn(acuity_listen(db));
    // Spawn Ethereum task.
    let ethereum_task = tokio::spawn(ethereum_listen());
    // Wait to exit.
    let _result = join!(websockets_task, acuity_task, ethereum_task);
}

async fn acuity_listen(db: DB) {
    let client = ClientBuilder::<AcuityRuntime>::new()
        .register_type_size::<[u8; 16]>("AcuityOrderId")
        .register_type_size::<[u8; 16]>("AcuityAssetId")
        .register_type_size::<[u8; 32]>("AcuityForeignAddress")
        .register_type_size::<[u8; 32]>("AcuityHashedSecret")
        .register_type_size::<[u8; 32]>("AcuitySecret")
        .register_type_size::<u64>("Timestamp")
        .register_type_size::<[u8; 20]>("EthereumAddress")
        .set_url("ws://127.0.0.1:9946").build().await.unwrap();

    let mut blocks = client.subscribe_blocks().await.unwrap();

    loop {
        let block = blocks.next().await.unwrap().unwrap();
        println!("Acuity block: {:?}", block.number);
        let block = client.block_hash(Some(block.number.into())).await.unwrap().unwrap();

        let sub = client.subscribe_events().await.unwrap();
        let decoder = client.events_decoder();
        let mut sub = EventSubscription::<AcuityRuntime>::new(sub, decoder);
        sub.filter_block(block);

        loop {
            let raw = sub.next().await;
            // Pattern match to retrieve the value
            match raw {
                Some(event) => {
                    let event = event.unwrap();
                    if event.module != "AtomicSwap" { continue; }

                    match event.variant.as_str() {
                        "AddToOrder" => {
                            let event = AddToOrderEvent::<AcuityRuntime>::decode(&mut &event.data[..]).unwrap();
                            println!("AddToOrderEvent: {:?}", event);
                            let order = Order {
                                seller: event.seller.encode(),
                                asset_id: event.asset_id,
                                price: event.price,
                                foreign_address: event.foreign_address,
                            };
                            let order_id = order.get_order_id();
                            println!("order_id: {:?}", order_id);
                            db.put_cf(db.cf_handle("order_static").unwrap(), order_id, bincode::serialize(&order).unwrap()).unwrap();
                        },
                        "RemoveFromOrder" => {
                            let event = RemoveFromOrderEvent::<AcuityRuntime>::decode(&mut &event.data[..]).unwrap();
                            println!("RemoveFromOrderEvent: {:?}", event);
                            let order = Order {
                                seller: event.seller.encode(),
                                asset_id: event.asset_id,
                                price: event.price,
                                foreign_address: event.foreign_address,
                            };
                            let order_id = order.get_order_id();
                            println!("order_id: {:?}", order_id);
                        },
                        "LockSell" => {
                            let event = LockSellEvent::<AcuityRuntime>::decode(&mut &event.data[..]).unwrap();
                            println!("LockSellEvent: {:?}", event);
                        },
                        "UnlockSell" => {
                            let event = UnlockSellEvent::<AcuityRuntime>::decode(&mut &event.data[..]).unwrap();
                            println!("UnlockSellEvent: {:?}", event);
                        },
                        "TimeoutSell" => {
                            let event = TimeoutSellEvent::decode(&mut &event.data[..]).unwrap();
                            println!("TimeoutSellEvent: {:?}", event);
                        },
                        "LockBuy" => {
                            let event = LockBuyEvent::<AcuityRuntime>::decode(&mut &event.data[..]).unwrap();
                            println!("LockBuyEvent: {:?}", event);
                        },
                        "UnlockBuy" => {
                            let event = UnlockBuyEvent::decode(&mut &event.data[..]).unwrap();
                            println!("UnlockBuyEvent: {:?}", event);
                        },
                        "TimeoutBuy" => {
                            let event = TimeoutBuyEvent::decode(&mut &event.data[..]).unwrap();
                            println!("TimeoutBuyEvent: {:?}", event);
                        },
                        _ => println!("variant: {:?}", event.variant),
                    }
                },
                None    => break,
            }
        }
    }
}

async fn ethereum_listen() {
    let ws = web3::transports::WebSocket::new("wss://mainnet.infura.io/ws/v3/9aa3d95b3bc440fa88ea12eaa4456161").await.unwrap();
    let web3 = web3::Web3::new(ws);
    let mut sub = web3.eth_subscribe().subscribe_new_heads().await.unwrap();

    println!("Got subscription id: {:?}", sub.id());

    (&mut sub)
        .for_each(|x| {
            println!("Ethereum block: {:?}", x.unwrap().number.unwrap());
            future::ready(())
        })
        .await;
}

async fn handle_connection(raw_stream: TcpStream, addr: SocketAddr) {
    println!("Incoming TCP connection from: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr);
}

async fn websockets_listen() {
    let addr = "127.0.0.1:8080".to_string();

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, addr));
    }
}
