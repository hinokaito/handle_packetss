'use client';

import { useState, useCallback } from 'react';
import type { StageConfig, StageListItem } from '@/types/stage';

// =============================================================================
// Stage API Hook - GoサーバーからステージJSONを取得
// =============================================================================

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';

export interface UseStageApiReturn {
  /** ステージ一覧を取得 */
  fetchStageList: () => Promise<StageListItem[]>;
  /** 特定ステージの詳細を取得 */
  fetchStage: (stageId: string) => Promise<StageConfig | null>;
  /** ローディング状態 */
  isLoading: boolean;
  /** エラーメッセージ */
  error: string | null;
  /** 現在ロード中のステージ */
  currentStage: StageConfig | null;
  /** ステージ一覧 */
  stageList: StageListItem[];
}

export function useStageApi(): UseStageApiReturn {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [currentStage, setCurrentStage] = useState<StageConfig | null>(null);
  const [stageList, setStageList] = useState<StageListItem[]>([]);

  /**
   * ステージ一覧を取得
   */
  const fetchStageList = useCallback(async (): Promise<StageListItem[]> => {
    setIsLoading(true);
    setError(null);

    try {
      const response = await fetch(`${API_BASE_URL}/api/stages`);
      
      if (!response.ok) {
        throw new Error(`Failed to fetch stage list: ${response.status}`);
      }

      const data: StageListItem[] = await response.json();
      setStageList(data);
      console.log('[useStageApi] Fetched stage list:', data.length, 'stages');
      return data;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Unknown error';
      setError(message);
      console.error('[useStageApi] Error fetching stage list:', err);
      return [];
    } finally {
      setIsLoading(false);
    }
  }, []);

  /**
   * 特定ステージの詳細を取得
   */
  const fetchStage = useCallback(async (stageId: string): Promise<StageConfig | null> => {
    setIsLoading(true);
    setError(null);

    try {
      const response = await fetch(`${API_BASE_URL}/api/stages/${stageId}`);
      
      if (!response.ok) {
        if (response.status === 404) {
          throw new Error(`Stage not found: ${stageId}`);
        }
        throw new Error(`Failed to fetch stage: ${response.status}`);
      }

      const data: StageConfig = await response.json();
      setCurrentStage(data);
      console.log('[useStageApi] Fetched stage:', data.meta.title);
      return data;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Unknown error';
      setError(message);
      console.error('[useStageApi] Error fetching stage:', err);
      return null;
    } finally {
      setIsLoading(false);
    }
  }, []);

  return {
    fetchStageList,
    fetchStage,
    isLoading,
    error,
    currentStage,
    stageList,
  };
}

