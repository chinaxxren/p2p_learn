pub mod behaviour;
pub mod event;
pub mod protocol;

use std::{error::Error, iter};

use libp2p::{
    identity::{self, ed25519},
    kad::{store::MemoryStore, Kademlia},
    request_response::{ProtocolSupport, RequestResponse},
    swarm::SwarmBuilder,
};
pub use protocol::*;
use tokio::sync::mpsc::{self, Receiver};

use crate::client::Client;

use self::{
    behaviour::ComposedBehaviour,
    event::{Event, EventLoop},
};

// 创建一个新的网络客户端和事件循环系统
//
// 此函数负责初始化一个网络客户端，包括生成密钥对、创建节点ID、构建网络层管理组件Swarm，
// 以及创建命令和事件的通信渠道。它返回一个包含客户端、事件接收器和事件循环系统的元组。
//
// 参数:
// - secret_key_seed: 用于生成密钥对的种子。如果未提供，则自动生成密钥对。
//
// 返回值:
// - Result: 包含客户端、事件接收器和事件循环系统的元组，或错误信息。
pub async fn new(
    secret_key_seed: Option<u8>,
) -> Result<(Client, Receiver<Event>, EventLoop), Box<dyn Error>> {

    // 创建密钥对
    // 根据提供的种子（如果有）生成一个Ed25519密钥对，用于网络通信的身份验证。
    let id_keys = match secret_key_seed {
        Some(seed) => {
            let mut bytes = [0u8; 32];
            bytes[0] = seed;
            let secret_key = ed25519::SecretKey::from_bytes(&mut bytes).expect(
                "this returns `Err` only if the length is wrong; the length is correct; qed",
            );
            identity::Keypair::Ed25519(secret_key.into())
        }
        None => identity::Keypair::generate_ed25519(),
    };

    // 根据公钥生成节点ID
    // 节点ID是网络中唯一标识一个节点的标识符，由公钥派生而来。
    let peer_id = id_keys.public().to_peer_id();

    // 构建网络层管理组件Swarm
    // Swarm是Libp2p中用于管理网络连接的核心组件。这里我们构建了一个包含Kademlia和RequestResponse行为的Swarm。
    let transport = libp2p::development_transport(id_keys).await?;

    // RequestResponse是Libp2p中的一个请求-响应协议，用于在网络中进行双向通信。
    let request_response = RequestResponse::new(
        // 自定义的文件交换协议，用于在网络中传输文件。
        FileExchangeCodec(),
        // 支持的协议列表，这里只支持文件交换协议。
        iter::once((FileExchangeProtocol(), ProtocolSupport::Full)),
        // 自定义的事件处理器，用于处理网络事件。
        Default::default(),
    );

    // Kademlia是Libp2p中的一个分布式哈希表协议，用于查找和维护网络中的节点。
    let kademlia = Kademlia::new(peer_id, MemoryStore::new(peer_id));

    // 构建自定义的网络行为
    let behaviour = ComposedBehaviour {
        kademlia,
        request_response,
    };

    // 创建网络层管理组件Swarm
    let swarm = SwarmBuilder::new(
        transport, 
        behaviour, peer_id,
    )
    .build();

    // 创建命令发送器和接收器
    let (command_sender, command_receiver) = mpsc::channel(1);

    // 事件发送器和接收器
    let (event_sender, event_receiver) = mpsc::channel(1);

    // 返回客户端、事件接收器和事件循环系统的元组
    Ok((
        Client::new(command_sender),
        event_receiver,
        EventLoop::new(swarm, command_receiver, event_sender),
    ))
}
