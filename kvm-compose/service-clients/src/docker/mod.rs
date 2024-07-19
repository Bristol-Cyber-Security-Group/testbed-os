use anyhow::{bail, Context};
use tokio::net::UnixStream;
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// This is a limited implementation for a Docker client, we have a set of commands we want to run
/// against the daemon and expected outputs. By no means is this supposed to be generic, so this
/// only implements the required GET and POST commands to support the testbed functionalities.
/// This is intended to be wrapped write a read/write lock to make it thread safe.
/// Unlikely it is needed but could create a connection pool wrapper to speed up communication but
/// the current throughput is plenty fast.
pub struct DockerUnixClient {
    pub conn_addr: String,
    pub socket: UnixStream,
}

impl DockerUnixClient {

    pub async fn new(conn_addr: &str) -> anyhow::Result<Self> {
        let socket = UnixStream::connect(conn_addr).await?;
        Ok(Self {
            conn_addr: conn_addr.to_string(),
            socket,

        })
    }

    /// Make a GET request to the socket, in these scenarios we always expect JSON
    async fn get_request(&mut self, request: &String) -> anyhow::Result<Value> {
        let request = format!("GET {request} HTTP/1.1\r\nHost: v1.43\r\n\r\n");
        // first send the request
        self.socket.write_all(request.as_bytes()).await?;

        // we need to read the buffer until we get to the end of the header, this is probably not
        // the right way to do this but it works and performance is OK
        let mut header_buffer = Vec::new();
        loop {
            let mut byte = [0u8; 1];
            let message_len = self.socket.read_exact(&mut byte).await?;
            if message_len == 0 {
                bail!("got to the end of the docker GET request response but did not get to the body, the header: {:?}", header_buffer);
            }
            header_buffer.push(byte[0]);
            if header_buffer.ends_with(b"\r\n\r\n") {
                break;
            }
            // TODO - could this run forever? or will the read exact error once there is nothing to
            //  read in the socket
        }
        // now we have the header, lets read the body
        let mut buffer = [0u8; 1024];
        let mut total_message = String::new();
        loop {
            let message_len = self.socket.read(&mut buffer).await?;
            let output = String::from_utf8_lossy(&buffer[..message_len]);
            total_message.push_str(&output);
            if message_len < 1024 {
                break;
            }
        }
        // make sure there is nothing before and after the json .. hacky.. yes
        // do first {
        let first_bracket = &total_message.find('{');
        let trimmed = if let Some(first) = first_bracket {
            total_message.get(*first..total_message.len())
                .context("trimming response body from start to first {")?
        } else {
            &total_message
        };
        // do last } and add +1 to index to not delete last }
        let last_bracket = &trimmed.rfind('}');
        let trimmed= if let Some(last) = last_bracket {
            trimmed.get(0..*last+1)
                .context("trimming response body from end to last }")?
        } else {
            trimmed
        };

        let json: Value = serde_json::from_str(trimmed)?;

        Ok(json)
    }

    // /// Make a GET request to the socket that returns a stream of responses from the docker daemon
    // async fn get_request_stream(&mut self, request: &String) -> anyhow::Result<Value> {
    //     let request = format!("GET {request} HTTP/1.1\r\nHost: v1.43\r\n\r\n");
    //     // first send the request
    //     self.socket.write_all(request.as_bytes()).await?;
    //
    //     // we need to read the buffer until we get to the end of the header, this is probably not
    //     // the right way to do this but it works and performance is OK
    //     let mut header_buffer = Vec::new();
    //     loop {
    //         let mut byte = [0u8; 1];
    //         let message_len = self.socket.read_exact(&mut byte).await?;
    //         if message_len == 0 {
    //             bail!("got to the end of the docker GET request response but did not get to the body, the header: {:?}", header_buffer);
    //         }
    //         header_buffer.push(byte[0]);
    //         if header_buffer.ends_with(b"\r\n\r\n") {
    //             break;
    //         }
    //         // TODO - could this run forever? or will the read exact error once there is nothing to
    //         //  read in the socket
    //     }
    //     let header = String::from_utf8_lossy(header_buffer.as_slice());
    //     println!("@ header = {header:?}");
    //     // now we have the header, lets read the body continuously
    //     let mut total_message = Vec::new();
    //     loop {
    //         let mut byte = [0u8; 1];
    //         let message_len = self.socket.read_exact(&mut byte).await?;
    //         // if message_len == 0 {
    //         //     bail!("got to the end of the docker GET request response but did not get to the body, the header: {:?}", header_buffer);
    //         // }
    //         total_message.push(byte[0]);
    //         // \n\r\n613\r\n appears to be the delimiter between the stats json in the stream
    //         if total_message.ends_with(b"\n\r\n613\r\n") {
    //             let res = String::from_utf8_lossy(total_message.as_slice());
    //             println!("@ end of one stats frame:\n{res:?}");
    //             total_message.clear();
    //         }
    //     }
    //
    //     todo!()
    // }

    // async fn post_request()

    pub async fn get_version(&mut self) -> anyhow::Result<String> {
        let version = self.get_request(&"/version".to_string()).await?;
        Ok(version["ApiVersion"].to_string())
    }

    /// This gets the guests stats, however the docker daemon will hold the response for about 1s
    /// to let it sample the resource usage to provide an average
    pub async fn get_guest_stats(&mut self, guest_name: &String) -> anyhow::Result<Value> {
        let request = format!("/containers/{guest_name}/stats?stream=false");
        let stats = self.get_request(&request).await?;
        Ok(stats)
    }

    /// Get the uuid from the docker inspect endpoint
    pub async fn inspect_guest(&mut self, guest_name: &String) -> anyhow::Result<Value> {
        let request = format!("/containers/{guest_name}/json");
        let inspect = self.get_request(&request).await?;
        Ok(inspect)
    }

    /// This function will get the stats from the filesystem rather than the docker socket. This is
    /// an alternative method to the socket specifically for guest statistics as the time it takes
    /// to get the statistics from the docker api is too slow.
    /// The documentation for the stats is https://docs.docker.com/config/containers/runmetrics/#control-groups
    /// See also https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html
    pub async fn get_guest_stats_filesystem() -> anyhow::Result<Value> {

        todo!()
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO - turn these into integration tests that only run if docker service is running, and then
    //  also create containers on demand to test some of these commands

    async fn get_docker_client() -> anyhow::Result<DockerUnixClient> {
        // this will fail all tests if docker socket is not up
        DockerUnixClient::new("/var/run/docker.sock").await
    }

    #[tokio::test]
    async fn test_connect_to_docker_socket() -> anyhow::Result<()> {
        let _ = get_docker_client().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_get_stats_container() -> anyhow::Result<()> {
        let mut client = get_docker_client().await?;
        let _stats = client.get_guest_stats(&"resource_monitoring-proxy-1".to_string()).await?;
        // println!("{:?}", stats);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_stats_container_fail() -> anyhow::Result<()> {
        let mut client = get_docker_client().await?;
        let stats = client.get_guest_stats(&"resource_monitoring-proxy-2".to_string()).await?;
        assert_eq!(stats["message"].to_string(), "\"No such container: resource_monitoring-proxy-2\"".to_string());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_docker_version() -> anyhow::Result<()> {
        let mut client = get_docker_client().await?;

        // just test we get a string response, since the version can change dont fix it in the test
        client.get_version().await?;

        Ok(())
    }


    #[tokio::test]
    async fn test_multiple_requests() -> anyhow::Result<()> {
        // for this test, the previous version of the docker client did not play nice with a second
        // request, it wasn't clear if the socket connection was being dropped hence the opening and
        // closing of the contexts below

        let mut client = get_docker_client().await?;

        {
            let _version = client.get_version().await?;
        }
        // again
        {
            let _version = client.get_version().await?;
        }
        Ok(())
    }

    // #[tokio::test]
    // async fn test_stats_stream() -> anyhow::Result<()> {
    //     let mut client = get_docker_client().await?;
    //     let request = format!("/containers/signal-client3/stats?stream=true");
    //     client.get_request_stream(&request).await?;
    //     Ok(())
    // }

    // #[tokio::test]
    // async fn test_get_docker_uuid() -> anyhow::Result<()> {
    //     let mut client = get_docker_client().await?;
    //     let inspect = client.get_guest_uuid(&"ovn-client3".to_string()).await?;
    //
    //     Ok(())
    // }

}
