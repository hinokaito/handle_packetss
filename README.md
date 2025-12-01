[日本語](README.ja.md)

# handle_packetss

**Infrastructure as a Game**\
Real-time packet traffic simulation using Rust (Wasm), WebGL, and Go (QUIC).

This project visualizes the flow of 100,000+ network packets and simulates load balancing algorithms in a browser-based environment.

## Concept
- **Massive Scale:** Visualize high-throughput traffic using WebGL instancing.
- **Low Level:** Custom binary protocols over QUIC (WebTransport).
- **Simulation:** Real-time feedback loop between Go backend and Wasm frontend.

## Tech Stack
- **Frontend:** Rust (Wasm), wgpu (WebGPU/WebGL), TypeScript
- **Backend:** Go, QUIC-go
- **Protocol:** WebTransport

## usage

### backend
```bash
cd server
go run main.go
```

### frontend
```bash
cd web
python -m http.server 8080
```


## License
**MIT License.**
This is a portfolio project.

