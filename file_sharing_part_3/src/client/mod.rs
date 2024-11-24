pub mod command;

use std::{collections::HashSet, error::Error};

use libp2p::{request_response::ResponseChannel, Multiaddr, PeerId};
use tokio::sync::{
    mpsc::{self, Sender},
    oneshot,
};

use crate::network::FileResponse;

pub use self::command::Command;

// 用于发送命令的Client
#[derive(Clone)]
pub struct Client {
    // 将命令发送到mpsc通道
    sender: mpsc::Sender<Command>,
}

impl Client {
    pub fn new(sender: Sender<Command>) -> Client {
        Client { sender }
    }

    /// 异步启动监听指定的多地址
    ///
    /// 该函数通过发送一个启动监听的命令给命令接收者，并等待确认来启动监听进程
    /// 如果命令接收者被意外丢弃，函数将返回错误
    ///
    /// # 参数
    /// * `addr` - 要监听的多地址
    ///
    /// # 返回
    /// * `Result<(), Box<dyn Error + Send>>` - 如果监听成功启动，则返回Ok(())；否则返回错误
    pub async fn start_listening(&mut self, addr: Multiaddr) -> Result<(), Box<dyn Error + Send>> {
        // 创建一个一次性通道，用于接收启动监听的结果
        let (sender, receiver) = oneshot::channel();

        // 发送启动监听的命令，并期望命令接收者不会被丢弃
        self.sender
            .send(Command::StartListening { addr, sender })
            .await
            .expect("Command receiver not to be dropped.");

        // 等待接收启动监听的结果，并期望发送者不会被丢弃
        receiver.await.expect("Sender not to be dropped.")
    }

    /// 异步拨号函数，用于发起与指定对等节点的连接请求
    ///
    /// # 参数
    ///
    /// * `peer_id` - 对等节点的唯一标识符
    /// * `peer_addr` - 对等节点的网络地址，使用Multiaddr格式
    ///
    /// # 返回
    ///
    /// * `Ok(())` - 连接请求成功发起
    /// * `Err` - 连接请求失败，返回一个动态错误类型
    pub async fn dial(
        &mut self,
        peer_id: PeerId,
        peer_addr: Multiaddr,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 创建一个一次性通道，用于接收命令执行的结果
        let (sender, receiver) = oneshot::channel();

        // 发送拨号命令到对等节点管理器
        self.sender
            .send(Command::Dial {
                peer_id,
                peer_addr,
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");

        // 等待命令执行结果，并处理接收结果
        receiver.await.expect("Sender not to be dropped.")
    }

    /// 异步启动文件提供服务
    ///
    /// 该函数通过发送命令到文件提供者，启动文件的提供服务它使用异步通道来同步操作，
    /// 确保命令发送后才会继续执行，以避免在文件提供服务启动前进行其他操作
    ///
    /// # 参数
    ///
    /// * `file_name` - 一个字符串，表示需要提供的文件名
    ///
    /// # 期望
    ///
    /// 期望命令接收者不会被丢弃，以确保命令能够被接收和处理此外，还期望在文件提供服务启动前，
    /// 命令发送者不会被丢弃，以确保命令完整发送
    pub async fn start_providing(&mut self, file_name: String) {
        // 创建一个一次性通道，用于接收命令执行的结果
        let (sender, receiver) = oneshot::channel();

        // 发送启动文件提供服务的命令，包括文件名和结果接收者
        self.sender
            .send(Command::StartProviding { file_name, sender })
            .await
            .expect("Command receiver not to be dropped.");

        // 等待命令执行结果，确保文件提供服务已启动
        receiver.await.expect("Sender not to be dropped.");
    }

    /// 异步获取文件提供者集合
    ///
    /// 该函数通过发送命令请求来获取指定文件名对应的提供者集合(PeerId的HashSet)。
    /// 它使用一次性通道(oneshot::channel)来接收响应，确保命令处理后能够接收到结果。
    ///
    /// # 参数
    /// - `file_name`: 需要查询的文件名
    ///
    /// # 返回
    /// - `HashSet<PeerId>`: 文件的提供者集合
    ///
    /// # 错误处理
    /// - 如果命令接收者被丢弃，`send` 方法会 panic。
    /// - 如果发送者被丢弃，`await` 方法会 panic。
    pub async fn get_providers(&mut self, file_name: String) -> HashSet<PeerId> {
        // 创建一次性通道，用于接收文件提供者信息
        let (sender, receiver) = oneshot::channel();

        // 发送获取提供者的命令，包含文件名和回传结果的发送端
        self.sender
            .send(Command::GetProviders { file_name, sender })
            .await
            .expect("Command receiver not to be dropped.");

        // 等待并接收提供者信息，如果发送者被丢弃，则会 panic
        receiver.await.expect("Sender not to be dropped.")
    }

    /// 异步请求文件
    ///
    /// 该函数通过发送命令请求从指定的对等端请求文件它使用一次性通道来接收响应
    /// 主要用于在分布式网络中从其他节点获取文件
    ///
    /// # 参数
    ///
    /// * `peer` - 指定的对等端ID，表示从哪个节点请求文件
    /// * `file_name` - 要请求的文件名字符串，表示需要获取的文件的名称
    ///
    /// # 返回
    ///
    /// * `Ok(String)` - 如果文件成功接收到，则返回文件内容的字符串
    /// * `Err(Box<dyn Error + Send>)` - 如果文件接收失败，则返回一个装箱的错误类型
    ///
    /// # 错误
    ///
    /// 可能的错误包括但不限于：
    /// * 发送命令时发生错误
    /// * 接收文件内容时发生错误
    /// * 对等端或文件不存在
    pub async fn request_file(
        &mut self,
        peer: PeerId,
        file_name: String,
    ) -> Result<String, Box<dyn Error + Send>> {
        // 创建一个一次性通道，用于接收文件内容
        let (sender, receiver) = oneshot::channel();

        // 发送请67求文件的命令，包含文件名、对等端ID和用于接收文件内容的发送端
        self.sender
            .send(Command::RequestFile {
                file_name,
                peer,
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");

        // 等待并接收文件内容，如果发送端已被丢弃，则返回错误
        receiver.await.expect("Sender not be dropped.")
    }

    #[allow(dead_code)]
    /// 异步处理文件响应请求
    ///
    /// 该函数用于将文件作为响应发送到请求者
    /// 它通过内部的sender将一个包含文件路径和响应通道的命令发送出去
    ///
    /// # 参数
    /// - `file`: 一个字符串，表示要响应的文件路径
    /// - `channel`: 一个响应通道，用于发送文件响应结果
    ///
    /// # 期望
    /// 期望命令接收者不会被丢弃如果接收者被丢弃，发送操作将失败，并产生一个panic
    pub async fn respond_file(&mut self, file: String, channel: ResponseChannel<FileResponse>) {
        self.sender
            .send(Command::RespondFile { file, channel })
            .await
            .expect("Command receiver not to be dropped.");
    }
}
