package main

import (
	"encoding/json"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"strings"
)

// =============================================================================
// DATA STRUCTURES
// =============================================================================

// StageConfig はステージ全体の設定を表す
type StageConfig struct {
	Meta  Meta      `json:"meta"`
	Map   MapConfig `json:"map"`
	Waves []Wave    `json:"waves"`
}

// Meta はステージのメタ情報
type Meta struct {
	Title       string  `json:"title"`
	Description string  `json:"description"`
	Budget      int     `json:"budget"`
	SLATarget   float64 `json:"sla_target"`
}

// MapConfig はマップ設定（固定ノードなど）
type MapConfig struct {
	FixedNodes []FixedNode `json:"fixed_nodes"`
}

// FixedNode は固定配置されるノード（Gateway等）
type FixedNode struct {
	ID   string `json:"id"`
	Type string `json:"type"`
	X    int    `json:"x"`
	Y    int    `json:"y"`
}

// Wave はパケット出現パターン
type Wave struct {
	TimeStartMs int     `json:"time_start_ms"`
	SourceID    string  `json:"source_id"`
	Count       int     `json:"count"`
	DurationMs  int     `json:"duration_ms"`
	PacketType  string  `json:"packet_type"`
	Speed       float64 `json:"speed"`
}

// StageListItem はステージ一覧用の簡易情報
type StageListItem struct {
	ID          string `json:"id"`
	Title       string `json:"title"`
	Description string `json:"description"`
}

// =============================================================================
// CORS MIDDLEWARE
// =============================================================================

func corsMiddleware(next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.Header().Set("Access-Control-Allow-Methods", "GET, OPTIONS")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type")

		if r.Method == "OPTIONS" {
			w.WriteHeader(http.StatusOK)
			return
		}

		next(w, r)
	}
}

// =============================================================================
// STAGE FILE OPERATIONS
// =============================================================================

const stagesDir = "stages"

// loadStageConfig はJSONファイルからステージ設定を読み込む
func loadStageConfig(stageID string) (*StageConfig, error) {
	filename := filepath.Join(stagesDir, stageID+".json")
	data, err := os.ReadFile(filename)
	if err != nil {
		return nil, err
	}

	var config StageConfig
	if err := json.Unmarshal(data, &config); err != nil {
		return nil, err
	}

	return &config, nil
}

// listStages はstagesディレクトリ内の全ステージを一覧取得
func listStages() ([]StageListItem, error) {
	entries, err := os.ReadDir(stagesDir)
	if err != nil {
		return nil, err
	}

	var stages []StageListItem
	for _, entry := range entries {
		if entry.IsDir() || !strings.HasSuffix(entry.Name(), ".json") {
			continue
		}

		stageID := strings.TrimSuffix(entry.Name(), ".json")
		config, err := loadStageConfig(stageID)
		if err != nil {
			log.Printf("Warning: failed to load stage %s: %v", stageID, err)
			continue
		}

		stages = append(stages, StageListItem{
			ID:          stageID,
			Title:       config.Meta.Title,
			Description: config.Meta.Description,
		})
	}

	return stages, nil
}

// =============================================================================
// API HANDLERS
// =============================================================================

// handleGetStages は GET /api/stages - ステージ一覧を返す
func handleGetStages(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	stages, err := listStages()
	if err != nil {
		log.Printf("Error listing stages: %v", err)
		http.Error(w, "Failed to list stages", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(stages)
}

// handleGetStage は GET /api/stages/{id} - 特定ステージの詳細を返す
func handleGetStage(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// パスから stage ID を抽出: /api/stages/{id}
	path := strings.TrimPrefix(r.URL.Path, "/api/stages/")
	stageID := strings.TrimSpace(path)

	if stageID == "" {
		http.Error(w, "Stage ID is required", http.StatusBadRequest)
		return
	}

	config, err := loadStageConfig(stageID)
	if err != nil {
		if os.IsNotExist(err) {
			http.Error(w, "Stage not found", http.StatusNotFound)
		} else {
			log.Printf("Error loading stage %s: %v", stageID, err)
			http.Error(w, "Failed to load stage", http.StatusInternalServerError)
		}
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(config)
}

// =============================================================================
// ROUTER
// =============================================================================

func setupRoutes() {
	// /api/stages - 一覧
	http.HandleFunc("/api/stages", corsMiddleware(handleGetStages))

	// /api/stages/{id} - 詳細
	http.HandleFunc("/api/stages/", corsMiddleware(handleGetStage))
}

// =============================================================================
// MAIN ENTRY POINT
// =============================================================================

func main() {
	// stagesディレクトリの存在確認
	if _, err := os.Stat(stagesDir); os.IsNotExist(err) {
		log.Printf("Warning: stages directory '%s' does not exist", stagesDir)
	}

	setupRoutes()

	addr := ":8080"
	log.Printf("REST API server starting on %s", addr)
	log.Printf("Endpoints:")
	log.Printf("  GET http://localhost%s/api/stages      - Stage list", addr)
	log.Printf("  GET http://localhost%s/api/stages/{id} - Stage detail", addr)

	if err := http.ListenAndServe(addr, nil); err != nil {
		log.Fatal("ListenAndServe: ", err)
	}
}
