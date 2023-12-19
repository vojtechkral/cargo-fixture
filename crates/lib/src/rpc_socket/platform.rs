// Unix
#[cfg(all(unix, feature = "smol"))]
pub use smol::net::unix::{UnixListener, UnixStream};
#[cfg(all(unix, feature = "tokio"))]
pub use tokio::net::{UnixListener, UnixStream};

// Windows  TODO:
#[cfg(windows)]
pub use uds_windows::{UnixListener, UnixStream}; // https://docs.rs/uds_windows/latest/uds_windows/struct.UnixListener.html
                                                 // TODO: will need to be wrapped for async? Or converted to TcpStream?

// Common
#[cfg(feature = "smol")]
pub use smol::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(feature = "tokio")]
pub use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
