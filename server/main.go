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
type Packet struct {
	ID uint32  `json:"id"`
	X  float64 `json:"x"`
	Y  float64 `json:"y"`
}

// =============================================================================
// WEBSOCKET CONFIGURATION
// =============================================================================
var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool {
		return true
	},
}

// =============================================================================
// WEBSOCKET HANDLER
// =============================================================================
func handleWebSocket(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("WebSocket upgrade error: %v", err)
		return
	}
	defer conn.Close()
	log.Println("Client connected!")

	err = conn.WriteMessage(websocket.TextMessage, []byte("Hello"))
	if err != nil {
		log.Printf("Write error: %v", err)
		return
	}
	log.Println("Sent: Hello")

	const packetCount = 1000
	packets := make([]Packet, packetCount)
	for i := 0; i < packetCount; i++ {
		packets[i] = Packet{
			ID: uint32(i),
			X:  rand.Float64() * 800.0,
			Y:  rand.Float64() * 600.0,
		}
	}

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

	for {
		messageType, message, err := conn.ReadMessage()
		if err != nil {
			log.Printf("Read error: %v", err)
			break
		}

		log.Printf("Received: %s", message)

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
func encodePacketsBinary(packets []Packet) []byte {
	buf := new(bytes.Buffer)

	for _, p := range packets {
		binary.Write(buf, binary.LittleEndian, p.ID)
		x16 := uint16(p.X * 65535.0 / 800.0)
		binary.Write(buf, binary.LittleEndian, x16)
		y16 := uint16(p.Y * 65535.0 / 600.0)
		binary.Write(buf, binary.LittleEndian, y16)
	}

	return buf.Bytes()
}

// =============================================================================
// MAIN ENTRY POINT
// =============================================================================
func main() {
	http.HandleFunc("/ws", handleWebSocket)
	addr := ":8080"

	log.Printf("WebSocket server starting on %s", addr)
	log.Printf("Connect to ws://localhost%s/ws", addr)

	if err := http.ListenAndServe(addr, nil); err != nil {
		log.Fatal("ListenAndServe: ", err)
	}
}