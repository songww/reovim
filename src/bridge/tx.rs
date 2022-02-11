use pin_project::pin_project;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::{
    io::{AsyncWrite, WriteHalf},
    net::TcpStream,
    process::ChildStdin,
};

#[derive(Debug)]
#[pin_project(project = TxProj)]
pub enum Tx {
    Child(#[pin] ChildStdin),
    Tcp(#[pin] WriteHalf<TcpStream>),
}

impl futures::io::AsyncWrite for Tx {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        match self.project() {
            TxProj::Child(inner) => inner.poll_write(cx, buf),
            TxProj::Tcp(inner) => inner.poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        match self.project() {
            TxProj::Child(inner) => inner.poll_flush(cx),
            TxProj::Tcp(inner) => inner.poll_flush(cx),
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        match self.project() {
            TxProj::Child(inner) => inner.poll_shutdown(cx),
            TxProj::Tcp(inner) => inner.poll_shutdown(cx),
        }
    }
}

impl From<ChildStdin> for Tx {
    fn from(cs: ChildStdin) -> Tx {
        Tx::Child(cs)
    }
}

impl From<WriteHalf<TcpStream>> for Tx {
    fn from(ts: WriteHalf<TcpStream>) -> Tx {
        Tx::Tcp(ts)
    }
}
