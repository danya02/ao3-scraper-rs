# ao3-scraper-rs
Scraper for Archive of Our Own written in Rust

```
# This needs to be a SOCKS5 proxy server, allowing access to AO3.
apiVersion: v1
kind: Service
metadata:
  name: shadowsocks-client-service
  namespace: ao3
spec:
  ports:
    - name: web
      port: 1080
      targetPort: socks

  selector:
    app: shadowsocks-client
```