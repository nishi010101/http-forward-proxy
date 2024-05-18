#  http-forward-proxy

A basic http forward proxy written in rust, that tunnels the data from the client to the target server. 
Currently the data is encrypted and hence the proxy does not provide any insight into the request and the response but sends the data on your behalf to the target server. The future goal is to be able to inspect the data as well.


### **How to Run:**

```cargo run```
 The proxy server runs on 9000. Once its up and running you can configure web proxy on address 127.0.0.1 at port 9000. From then onwards all the traffic will be directed via the proxy.




