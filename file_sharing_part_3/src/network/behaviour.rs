use libp2p::{
    kad::{store::MemoryStore, Kademlia, KademliaEvent},
    request_response::{RequestResponse, RequestResponseEvent},
    NetworkBehaviour,
};

use super::protocol::{FileExchangeCodec, FileRequest, FileResponse};

// 组合Kademlia和请求-响应协议
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "ComposedEvent")]
pub struct ComposedBehaviour {
    
    // 用于请求-响应协议的行为
    pub request_response: RequestResponse<FileExchangeCodec>,
    
    // 用于Kademlia协议的行为
    pub kademlia: Kademlia<MemoryStore>,
}

// 网络行为事件
#[derive(Debug)]
pub enum ComposedEvent {
    
    // 请求-响应协议事件
    RequestResponse(RequestResponseEvent<FileRequest, FileResponse>),
    
    // Kademlia协议事件
    Kademlia(KademliaEvent),
}

impl From<RequestResponseEvent<FileRequest, FileResponse>> for ComposedEvent {
    fn from(event: RequestResponseEvent<FileRequest, FileResponse>) -> Self {
        ComposedEvent::RequestResponse(event)
    }
}

impl From<KademliaEvent> for ComposedEvent {
    fn from(event: KademliaEvent) -> Self {
        ComposedEvent::Kademlia(event)
    }
}
