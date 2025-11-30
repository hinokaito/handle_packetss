// =============================================================================
// Package: main
// =============================================================================
//
// WebSocket Server for Packet Traffic Simulation
//
// This is the backend server that handles real-time communication with the
// browser-based frontend. It uses WebSocket protocol to enable bidirectional
// streaming of packet simulation data.
//
// # Architecture Overview
//
//	┌─────────────────────────────────────────────────────────────────────────┐
//	│                              Browser                                    │
//	│  ┌─────────────────┐      ┌──────────────────┐                          │
//	│  │   JavaScript    │ ──── │   Rust/Wasm      │                          │
//	│  │   (WebSocket)   │      │   (Simulation)   │                          │
//	│  └────────┬────────┘      └──────────────────┘                          │
//	│           │                                                             │
//	└───────────┼─────────────────────────────────────────────────────────────┘
//	            │ WebSocket (ws://localhost:8080/ws)
//	            ▼
//	┌─────────────────────────────────────────────────────────────────────────┐
//	│                         This Go Server                                  │
//	│  - Handles WebSocket connections                                        │
//	│  - Streams packet simulation data                                       │
//	│  - Manages client state (TODO)                                          │
//	└─────────────────────────────────────────────────────────────────────────┘
//
// # Running the Server
//
//	$ cd server
//	$ go run main.go
//
// # Connecting from Browser
//
//	const ws = new WebSocket('ws://localhost:8080/ws');

package main

import (
	"bytes"
	"encoding/binary"
	"log"
	"math/rand"
	"net/http"
	"time"

	"github.com/gorilla/websocket"
)

// =============================================================================
// DATA STRUCTURES
// =============================================================================

// Packet represents a single packet in the simulation.
// This structure is serialized to JSON and sent to the browser.
//
// JSON format: {"id": 1, "x": 10.5, "y": 20.0}
type Packet struct {
	ID uint32  `json:"id"` // Unique identifier for the packet
	X  float64 `json:"x"`  // X coordinate (0.0 - 800.0)
	Y  float64 `json:"y"`  // Y coordinate (0.0 - 600.0)
}

// =============================================================================
// WEBSOCKET CONFIGURATION
// =============================================================================

// upgrader is responsible for upgrading HTTP connections to WebSocket connections.
//
// The WebSocket protocol starts as an HTTP request, then "upgrades" to a
// persistent bidirectional connection. This upgrader handles that transition.
//
// Configuration options:
//   - ReadBufferSize: Size of the read buffer (default: 4096)
//   - WriteBufferSize: Size of the write buffer (default: 4096)
//   - CheckOrigin: Function to validate the request origin
//
// Security Note:
// In production, CheckOrigin should validate that requests come from
// trusted domains. Currently set to allow all origins for development.
var upgrader = websocket.Upgrader{
	// CheckOrigin determines if the WebSocket handshake request is acceptable.
	//
	// This function receives the HTTP request and returns true if the
	// connection should be allowed, false otherwise.
	//
	// WARNING: `return true` allows connections from ANY origin.
	// This is convenient for development but INSECURE for production.
	//
	// Production example:
	//   CheckOrigin: func(r *http.Request) bool {
	//       origin := r.Header.Get("Origin")
	//       return origin == "https://yourdomain.com"
	//   },
	CheckOrigin: func(r *http.Request) bool {
		return true // Allow all origins (development only!)
	},
}

// =============================================================================
// WEBSOCKET HANDLER
// =============================================================================

// handleWebSocket processes incoming WebSocket connections.
//
// This is the main handler function that:
//  1. Upgrades the HTTP connection to WebSocket
//  2. Sends an initial greeting message
//  3. Enters a read loop to handle incoming messages
//  4. Echoes received messages back to the client (for testing)
//
// # Parameters
//
//   - w: http.ResponseWriter - Used to write the HTTP response (before upgrade)
//   - r: *http.Request - The incoming HTTP request containing upgrade headers
//
// # Connection Lifecycle
//
//	Client                          Server
//	  │                               │
//	  │ ── HTTP GET /ws ────────────> │  (1) Initial HTTP request
//	  │                               │
//	  │ <─ HTTP 101 Switching ─────── │  (2) Protocol upgrade
//	  │                               │
//	  │ <═══ WebSocket "Hello" ═════> │  (3) Server sends greeting
//	  │                               │
//	  │ <══════ Messages ═══════════> │  (4) Bidirectional messaging
//	  │                               │
//	  │ ── Close ───────────────────> │  (5) Connection closed
//	  │                               │
//
// # Error Handling
//
// The function handles several error cases:
//   - Upgrade failure: Logs error and returns (client gets HTTP error)
//   - Write failure: Logs error and closes connection
//   - Read failure: Logs error and exits the read loop (closes connection)
//
// # Future Enhancements
//
//   - Binary message support for efficient packet data transfer
//   - Client session management
//   - Pub/sub pattern for multiple simulation types
//   - Rate limiting and backpressure handling
func handleWebSocket(w http.ResponseWriter, r *http.Request) {
	// -------------------------------------------------------------------------
	// Step 1: Upgrade HTTP connection to WebSocket
	// -------------------------------------------------------------------------
	//
	// The Upgrade method:
	// 1. Validates the request headers (Upgrade: websocket, Connection: Upgrade)
	// 2. Performs the WebSocket handshake
	// 3. Returns a *websocket.Conn for bidirectional communication
	//
	// If upgrade fails, Upgrade() writes an HTTP error response automatically.
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("WebSocket upgrade error: %v", err)
		return
	}

	// Ensure the connection is closed when this function exits.
	// This prevents resource leaks if the client disconnects unexpectedly.
	defer conn.Close()

	// Log successful connection for debugging
	log.Println("Client connected!")

	// -------------------------------------------------------------------------
	// Step 2: Send initial greeting message (plain text)
	// -------------------------------------------------------------------------
	err = conn.WriteMessage(websocket.TextMessage, []byte("Hello"))
	if err != nil {
		log.Printf("Write error: %v", err)
		return
	}
	log.Println("Sent: Hello")

	// -------------------------------------------------------------------------
	// Step 2.5: Send packets as BINARY
	// -------------------------------------------------------------------------
	const packetCount = 100000

	// Generate random packets
	packets := make([]Packet, packetCount)
	for i := 0; i < packetCount; i++ {
		packets[i] = Packet{
			ID: uint32(i),
			X:  rand.Float64() * 800.0,
			Y:  rand.Float64() * 600.0,
		}
	}

	// Encode and send
	startEncode := time.Now()
	binaryData := encodePacketsBinary(packets)
	encodeDuration := time.Since(startEncode)

	err = conn.WriteMessage(websocket.BinaryMessage, binaryData)
	if err != nil {
		log.Printf("Binary write error: %v", err)
		return
	}

	log.Printf("Sent %d packets (%d bytes, %.2f KB) in %v",
		packetCount, len(binaryData), float64(len(binaryData))/1024, encodeDuration)

	// -------------------------------------------------------------------------
	// Step 3: Message read loop (main event loop)
	// -------------------------------------------------------------------------
	//
	// This infinite loop:
	// 1. Waits for a message from the client (blocking call)
	// 2. Logs the received message
	// 3. Echoes the message back to the client
	//
	// The loop exits when:
	// - Client closes the connection
	// - Network error occurs
	// - Server shuts down
	//
	// TODO: Future implementation will replace this simple echo with:
	// - Packet data generation and streaming
	// - Simulation control commands (start, stop, configure)
	// - Binary protocol parsing
	for {
		// ReadMessage blocks until a message is received.
		// Returns:
		//   - messageType: int (TextMessage=1, BinaryMessage=2, etc.)
		//   - message: []byte (the message payload)
		//   - err: error (nil on success, error on failure)
		messageType, message, err := conn.ReadMessage()
		if err != nil {
			// Common errors:
			// - websocket.CloseGoingAway: Client navigated away
			// - websocket.CloseNormalClosure: Client closed cleanly
			// - io.EOF: Connection lost
			log.Printf("Read error: %v", err)
			break
		}

		// Log the received message for debugging
		log.Printf("Received: %s", message)

		// Echo the message back to the client.
		// This is useful for testing round-trip latency and connection health.
		//
		// TODO: Replace this echo with actual simulation data:
		//
		// switch parseCommand(message) {
		// case "start":
		//     go streamPacketData(conn)
		// case "stop":
		//     stopStreaming()
		// case "config":
		//     updateConfig(message)
		// }
		err = conn.WriteMessage(messageType, message)
		if err != nil {
			log.Printf("Write error: %v", err)
			break
		}
	}
}

// =============================================================================
// BINARY ENCODING FUNCTION
// =============================================================================
//
// # Create a binary encoding function for the Packet struct
//
// Binary format (8 bytes per packet):
// ┌──────────────────────────────────────────────────┐
// │ ID (4 bytes) │ X (2 bytes) │ Y (2 bytes)         │
// └──────────────────────────────────────────────────┘
//
// func encodePackets(packets []Packet) []byte { ... }
// func decodePackets(data []byte) []Packet { ... }
func encodePacketsBinary(packets []Packet) []byte {
	buf := new(bytes.Buffer)

	for _, p := range packets {
		// ID (4 bytes, Little Endian)
		binary.Write(buf, binary.LittleEndian, p.ID)

		// X coordinate to uint16 (0-800 → 0-65535)
		x16 := uint16(p.X * 65535.0 / 800.0)
		binary.Write(buf, binary.LittleEndian, x16)

		// Y coordinate to uint16 (0-600 → 0-65535)
		y16 := uint16(p.Y * 65535.0 / 600.0)
		binary.Write(buf, binary.LittleEndian, y16)
	}

	return buf.Bytes()
}

// =============================================================================
// MAIN ENTRY POINT
// =============================================================================

// main is the entry point of the application.
//
// It sets up the HTTP server with WebSocket support and starts listening
// for incoming connections.
//
// # Server Configuration
//
//   - Address: :8080 (all interfaces, port 8080)
//   - Endpoint: /ws (WebSocket upgrade endpoint)
//
// # Usage
//
//	$ go run main.go
//	2024/01/01 12:00:00 WebSocket server starting on :8080
//	2024/01/01 12:00:00 Connect to ws://localhost:8080/ws
//
// # Future Enhancements
//
//   - HTTPS/TLS support for secure connections
//   - Configuration via environment variables or config file
//   - Graceful shutdown handling
//   - Health check endpoint (/health)
//   - Metrics endpoint (/metrics)
//   - WebTransport support (QUIC-based, more efficient than WebSocket)
func main() {
	// -------------------------------------------------------------------------
	// Route Configuration
	// -------------------------------------------------------------------------
	//
	// Register the WebSocket handler at the /ws endpoint.
	// Any HTTP request to /ws will be handled by handleWebSocket.
	//
	// The handler flow:
	//   GET /ws → handleWebSocket() → Upgrade to WebSocket → Bidirectional comm
	http.HandleFunc("/ws", handleWebSocket)

	// Server address configuration
	// ":8080" means listen on all network interfaces at port 8080
	//
	// Alternative formats:
	//   - "localhost:8080" - Only accept local connections
	//   - "192.168.1.100:8080" - Specific interface only
	//   - ":0" - Let OS assign a random available port
	addr := ":8080"

	// Log startup information
	log.Printf("WebSocket server starting on %s", addr)
	log.Printf("Connect to ws://localhost%s/ws", addr)

	// -------------------------------------------------------------------------
	// Start HTTP Server
	// -------------------------------------------------------------------------
	//
	// ListenAndServe starts an HTTP server that:
	// 1. Listens on the specified address
	// 2. Accepts incoming connections
	// 3. Routes requests to registered handlers
	//
	// This call blocks forever (or until an error occurs).
	// Common errors:
	//   - Port already in use
	//   - Permission denied (ports < 1024 require root on Unix)
	//   - Network interface not available
	if err := http.ListenAndServe(addr, nil); err != nil {
		log.Fatal("ListenAndServe: ", err)
	}
}

// =============================================================================
// TODO: FUTURE IMPLEMENTATIONS
// =============================================================================

// TODO: Implement packet simulation engine
//
// type Packet struct {
//     ID          uint64
//     Source      string
//     Destination string
//     Size        uint32
//     Timestamp   time.Time
// }
//
// func generatePackets(rate int) <-chan Packet { ... }

// TODO: Implement binary protocol for efficient data transfer
//
// Binary format (8 bytes per packet):
// ┌──────────────────────────────────────────────────┐
// │ ID (4 bytes) │ X (2 bytes) │ Y (2 bytes)         │
// └──────────────────────────────────────────────────┘
//
// func encodePackets(packets []Packet) []byte { ... }
// func decodePackets(data []byte) []Packet { ... }

// TODO: Implement WebTransport (QUIC) support
//
// WebTransport advantages over WebSocket:
// - Multiplexed streams (no head-of-line blocking)
// - UDP-based (lower latency for real-time data)
// - Native support in modern browsers
//
// func handleWebTransport(w http.ResponseWriter, r *http.Request) { ... }
