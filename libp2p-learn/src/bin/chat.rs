use anyhow::{Ok, Result};
use futures::StreamExt;
use libp2p::{
    core::upgrade,
    floodsub::{self, Floodsub, FloodsubEvent},
    identity,
    mdns::{Mdns, MdnsEvent},
    noise,
    swarm::{NetworkBehaviourEventProcess, SwarmBuilder, SwarmEvent},
    tcp::{GenTcpConfig, TokioTcpTransport},
    yamux, Multiaddr, NetworkBehaviour, PeerId, Transport,
};
use tokio::io::{self, AsyncBufReadExt};

// 自定义网络行为，组合floodsub和mDNS。
#[derive(NetworkBehaviour)]
#[behaviour(event_process = true)]
struct MyBehaviour {
    // Floodsub 是一种基于发布/订阅（Publish/Subscribe）模式的简单消息传递协议，
    // 通常用于去中心化网络中，如 IPFS（InterPlanetary File System）和 libp2p。
    // Floodsub 是一种广播协议，它通过将消息广播到所有连接的节点来实现消息的传播
    floodsub: Floodsub,
    // 它将主机名解析为 IP 地址。 在 libp2p 中，mDNS 用于发现网络上的其他节点。
    // 在 libp2p 中实现的网络行为 mDNS 将自动发现本地网络上的其他 libp2p 节点
    mdns: Mdns,
}

impl MyBehaviour {
    // 传入peerId，构建行为
    async fn new(id: PeerId) -> Result<Self> {
        Ok(Self {
            // floodsub协议初始化
            floodsub: Floodsub::new(id),
            // mDNS协议初始化
            mdns: Mdns::new(Default::default()).await?,
        })
    }
}

// 处理Floodsub网络行为事件
impl NetworkBehaviourEventProcess<FloodsubEvent> for MyBehaviour {
    // 当产生一个floodsub事件时，该方法被调用。
    fn inject_event(&mut self, message: FloodsubEvent) {
        // 显示接收到的消息及来源
        if let FloodsubEvent::Message(message) = message {
            println!(
                "收到消息: '{:?}' 来自 {:?}",
                String::from_utf8_lossy(&message.data),
                message.source
            );
        }
    }
}

// 处理mDNS网络行为事件
impl NetworkBehaviourEventProcess<MdnsEvent> for MyBehaviour {
    // 当产生一个mDNS事件时，该方法被调用。
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            
            // 发现新节点时，将节点添加到传播消息的节点列表中。
            MdnsEvent::Discovered(list) => {
                for (peer, _) in list {
                    self.floodsub.add_node_to_partial_view(peer);
                    println!("在网络中加入节点: {peer} ");
                }
            }

            // 当节点失效时，从传播消息的节点列表中删除一个节点。
            MdnsEvent::Expired(list) => {
                for (peer, _) in list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                        println!("从网络中删除节点: {peer} ");
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 生成密钥对
    let id_keys = identity::Keypair::generate_ed25519();

    // 基于密钥对的公钥，生成节点唯一标识peerId
    let peer_id = PeerId::from(id_keys.public());
    println!("节点ID: {peer_id}");

    // 创建noise密钥对
    let noise_keys = noise::Keypair::<noise::X25519Spec>::new().into_authentic(&id_keys)?;

    // 创建一个基于tokio的TCP传输层，使用noise进行身份验证。
    // 由于多了一层加密，所以使用yamux基于TCP流进行多路复用。
    // 最后，使用boxed()方法将其转换为BoxedTransport类型。
    // 最后，使用select_next_some()方法监听Swarm的事件。
    // select_next_some()方法用于从Swarm中选择下一个事件。

    // 首先，创建一个基于tokio的TCP传输层。
    // 然后，使用upgrade::Version::V1方法升级传输层的版本。
    // 接着，使用authenticate()方法进行身份验证。
    // 最后，使用multiplex()方法进行多路复用。
    // 最后，使用boxed()方法将其转换为BoxedTransport类型。
    // 最后，使用select_next_some()方法监听Swarm的事件。

    // 在 libp2p 中，"upgrade" 是指将一个普通的网络连接（如 TCP 连接）升级为一个安全的、多路复用的、加密的连接。
    
    // yamux
    // 建立连接：首先，两个节点之间建立一个底层连接（例如 TCP 连接）。
    // 创建流：在底层连接上，Yamux 可以创建多个逻辑流。每个流都有一个唯一的标识符。

    // Noise
    // 协议的核心是一个握手协议，用于在通信双方之间建立安全的加密通道
    let transport = TokioTcpTransport::new(GenTcpConfig::default().nodelay(true))
        .upgrade(upgrade::Version::V1)// 升级传输层的版本
        .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(yamux::YamuxConfig::default())
        .boxed();

    // 创建 Floodsub 主题
    let floodsub_topic = floodsub::Topic::new("chat");

    // 创建Swarm来管理节点网络及事件。
    let mut swarm = {
        let mut behaviour = MyBehaviour::new(peer_id).await?;

        // 订阅floodsub topic
        behaviour.floodsub.subscribe(floodsub_topic.clone());

        // 创建Swarm，传入transport、behaviour、peer_id。
        // 最后，使用executor()方法指定一个执行器，用于执行Swarm中的任务。
        // 最后，使用build()方法构建Swarm。
        // 最后，使用select_next_some()方法监听Swarm的事件。
        SwarmBuilder::new(transport, behaviour, peer_id)
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build()
    };

    // 指定一个远程节点，进行手动链接。
    if let Some(to_dial) = std::env::args().nth(1) {
        let addr: Multiaddr = to_dial.parse()?;
        swarm.dial(addr)?;
        println!("链接远程节点: {to_dial}");
    }

    // 从标准输入中读取消息
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    // 监听操作系统分配的端口
    // 0 表示操作系统分配一个端口。并返回一个Multiaddr。  
    swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse()?)?;

    loop {
        // 等待标准输入或Swarm事件发生。
        // 当标准输入有新的消息时，将其发布到订阅了floodsub topic的节点上。
        // 当Swarm有新的事件发生时，将其打印出来。
        // 最后，使用select_next_some()方法监听Swarm的事件。

        // 多任务等待：tokio::select! 允许你同时等待多个异步任务，并在其中一个任务完成时继续执行。
        // 非阻塞：tokio::select! 是非阻塞的，这意味着它不会阻塞当前线程，而是立即返回一个完成的任务。
        tokio::select! {
            line = stdin.next_line() => {
                let line = line?.expect("stdin closed");
                // 从标准输入中读取消息后，发布到订阅了floodsub topic的节点上。
                swarm.behaviour_mut().floodsub.publish(floodsub_topic.clone(), line.as_bytes());
            }
            // 当Swarm有新的事件发生时，将其打印出来。
            event = swarm.select_next_some() => {
                if let SwarmEvent::NewListenAddr { address, .. } = event {
                    println!("本地监听地址: {address}");
                }
            }
        }
    }
}
