use async_trait::async_trait;
use futures::{io, AsyncRead, AsyncWrite, AsyncWriteExt};
use libp2p::{
    core::{
        upgrade::{read_length_prefixed, write_length_prefixed},
        ProtocolName,
    },
    request_response::RequestResponseCodec,
};

#[derive(Debug, Clone)]

// 
pub struct FileExchangeProtocol();


#[derive(Clone)]
pub struct FileExchangeCodec();
#[derive(Debug, Clone, PartialEq, Eq)]

// 传输数据的编解码方式
pub struct FileRequest(pub String);

// 传输数据的编解码方式
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileResponse(pub String);

// 定义协议名称
impl ProtocolName for FileExchangeProtocol {
    
    // 返回协议名称
    fn protocol_name(&self) -> &[u8] {
        "/file-exchange/1".as_bytes()
    }
}

// 传输数据的编解码方式
#[async_trait]
impl RequestResponseCodec for FileExchangeCodec {
    type Protocol = FileExchangeProtocol;
    type Request = FileRequest;
    type Response = FileResponse;

    // 读请求
    async fn read_request<T>(
        &mut self,
        _: &FileExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        // 读取固定长度的字节
        let vec = read_length_prefixed(io, 1_000_000).await?;

        // 检查是否为空
        if vec.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        Ok(FileRequest(String::from_utf8(vec).unwrap()))
    }

    // 读取响应
    async fn read_response<T>(
        &mut self,
        _: &FileExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        // 读取固定长度的字节
        let vec = read_length_prefixed(io, 1_000_000).await?;

        // 检查是否为空
        if vec.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        Ok(FileResponse(String::from_utf8(vec).unwrap()))
    }

    // 写请求
    async fn write_request<T>(
        &mut self,
        _: &FileExchangeProtocol,
        io: &mut T,
        FileRequest(data): FileRequest,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        // 写入数据的长度
        write_length_prefixed(io, data).await?;

        // 关闭连接
        io.close().await?;

        Ok(())
    }

    // 写响应
    async fn write_response<T>(
        &mut self,
        _: &FileExchangeProtocol,
        io: &mut T,
        FileResponse(data): FileResponse,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {

        // 写入数据的长度
        write_length_prefixed(io, data).await?;

        // 关闭连接
        io.close().await?;

        Ok(())
    }
}
