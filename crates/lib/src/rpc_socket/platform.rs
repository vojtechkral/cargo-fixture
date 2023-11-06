// Unix
#[cfg(all(unix, feature = "smol"))]
pub use smol::net::unix::{UnixListener, UnixStream};
#[cfg(all(unix, feature = "tokio"))]
pub use tokio::net::{UnixListener, UnixStream};

// Common
#[cfg(feature = "smol")]
pub use smol::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(feature = "tokio")]
pub use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

// Windows
#[cfg(windows)]
pub use windows::*;

#[cfg(all(windows, feature = "smol"))]
mod windows {
    //! A wrapper around `uds_windows` using smol's `Async` feature.

    use std::{
        io,
        path::{Path, PathBuf},
    };

    use smol::{Async, Task};
    use uds_windows::SocketAddr;

    pub type UnixStream = Async<uds_windows::UnixStream>;

    #[derive(Debug)]
    pub struct UnixListener {
        inner: Async<uds_windows::UnixListener>,
    }

    impl UnixListener {
        pub fn bind(path: impl AsRef<Path>) -> io::Result<Self> {
            let inner = uds_windows::UnixListener::bind(path).and_then(Async::new)?;
            Ok(Self { inner })
        }

        pub async fn accept(&self) -> io::Result<(UnixStream, SocketAddr)> {
            let (socket, addr) = self.inner.read_with(|io| io.accept()).await?;
            let socket = Async::new(socket)?;
            Ok((socket, addr))
        }
    }

    pub trait UnixStreamExt: Sized {
        fn connect(path: PathBuf) -> Task<io::Result<Self>>;
    }

    impl UnixStreamExt for UnixStream {
        fn connect(path: PathBuf) -> Task<io::Result<Self>> {
            smol::unblock(|| uds_windows::UnixStream::connect(path).and_then(Async::new))
        }
    }
}

#[cfg(all(windows, feature = "tokio"))]
mod windows {
    //! Since Tokio doesn't have a counterpart to smol's `Async`, this is a bit more cumbersome.
    //!
    //! `accept()` and `connect()` are implemented `spawn_blocking()`, while
    //! async read and write on the Stream are implemented by turning the handle into
    //! `TcpStream` unsafely. This is not exposed and only read/write are performed,
    //! which should be compatible.

    use std::{
        io,
        net::TcpStream as StdTcpStream,
        os::windows::io::{FromRawSocket as _, IntoRawSocket as _},
        path::{Path, PathBuf},
        pin::Pin,
        sync::Mutex,
        task::{Context, Poll},
    };

    use tokio::{
        io::{AsyncRead, AsyncWrite, ReadBuf},
        net::TcpStream,
        pin,
        task::spawn_blocking,
    };
    use uds_windows::SocketAddr;

    #[derive(Debug)]
    pub struct UnixListener {
        sync: Mutex<Option<uds_windows::UnixListener>>,
    }

    impl UnixListener {
        pub fn bind(path: impl AsRef<Path>) -> io::Result<Self> {
            let sync = uds_windows::UnixListener::bind(path)
                .map(Some)
                .map(Mutex::new)?;
            Ok(Self { sync })
        }

        pub async fn accept(&self) -> io::Result<(UnixStream, SocketAddr)> {
            let mut lock = self.sync.lock().unwrap();
            let srv_socket = lock.take().unwrap();
            let (srv_socket, res) = spawn_blocking(move || {
                let res = srv_socket.accept();
                (srv_socket, res)
            })
            .await
            .unwrap();
            *lock = Some(srv_socket);

            let (acc_socket, addr) = res?;
            let acc_socket = UnixStream::new(acc_socket)?;
            Ok((acc_socket, addr))
        }
    }

    #[derive(Debug)]
    pub struct UnixStream {
        inner: TcpStream,
    }

    impl UnixStream {
        fn new(socket: uds_windows::UnixStream) -> io::Result<Self> {
            socket.set_nonblocking(true)?;
            let socket = socket.into_raw_socket();
            let socket = unsafe { StdTcpStream::from_raw_socket(socket) };
            let inner = TcpStream::from_std(socket)?;
            Ok(Self { inner })
        }

        pub async fn connect(path: PathBuf) -> io::Result<Self> {
            let socket = spawn_blocking(move || uds_windows::UnixStream::connect(path))
                .await
                .unwrap()?;
            Self::new(socket)
        }
    }

    impl AsyncRead for UnixStream {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            let socket = &mut self.as_mut().inner;
            pin!(socket);
            socket.poll_read(cx, buf)
        }
    }

    impl AsyncWrite for UnixStream {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            let socket = &mut self.as_mut().inner;
            pin!(socket);
            socket.poll_write(cx, buf)
        }

        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            let socket = &mut self.as_mut().inner;
            pin!(socket);
            socket.poll_flush(cx)
        }

        fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            let socket = &mut self.as_mut().inner;
            pin!(socket);
            socket.poll_shutdown(cx)
        }
    }
}
