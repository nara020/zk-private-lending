/**
 * WebSocket Hook
 *
 * 실시간 데이터 스트리밍을 위한 WebSocket 연결 관리 훅
 *
 * # Features
 * - 자동 재연결
 * - Heartbeat (keepalive)
 * - 메시지 구독/구독취소
 * - 연결 상태 관리
 */

import { useState, useEffect, useCallback, useRef } from 'react';

// WebSocket 메시지 타입
export interface WsMessage {
  type: string;
  data: unknown;
}

// Pool 상태 업데이트
export interface PoolStatusUpdate {
  total_collateral_eth: string;
  total_borrowed_usdc: string;
  available_liquidity: string;
  utilization_rate: number;
  current_interest_rate: number;
  eth_price: string;
  timestamp: number;
}

// 포지션 업데이트
export interface PositionUpdate {
  address: string;
  has_deposit: boolean;
  has_borrow: boolean;
  borrowed_amount: string;
  accrued_interest: string;
  total_debt: string;
  health_factor: number | null;
  timestamp: number;
}

// 가격 업데이트
export interface PriceUpdate {
  asset: string;
  price: string;
  change_24h: number;
  timestamp: number;
}

// 청산 경고
export interface LiquidationWarning {
  address: string;
  health_factor: number;
  threshold: number;
  message: string;
  urgency: 'Low' | 'Medium' | 'High' | 'Critical';
  timestamp: number;
}

// 연결 상태
type ConnectionStatus = 'connecting' | 'connected' | 'disconnected' | 'error';

// 훅 옵션
interface UseWebSocketOptions {
  url?: string;
  autoConnect?: boolean;
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
  heartbeatInterval?: number;
}

// 이벤트 핸들러
interface EventHandlers {
  onPoolStatus?: (data: PoolStatusUpdate) => void;
  onPositionUpdate?: (data: PositionUpdate) => void;
  onPriceUpdate?: (data: PriceUpdate) => void;
  onLiquidationWarning?: (data: LiquidationWarning) => void;
  onError?: (error: Error) => void;
  onConnect?: () => void;
  onDisconnect?: () => void;
}

/**
 * WebSocket 연결 훅
 *
 * @example
 * ```tsx
 * const { status, poolStatus, subscribe } = useWebSocket({
 *   url: 'ws://localhost:3001/ws',
 *   onPoolStatus: (data) => console.log('Pool:', data),
 * });
 *
 * useEffect(() => {
 *   subscribe('pool_status');
 * }, []);
 * ```
 */
export function useWebSocket(options: UseWebSocketOptions & EventHandlers = {}) {
  const {
    url = import.meta.env.VITE_WS_URL || 'ws://localhost:3001/ws',
    autoConnect = true,
    reconnectInterval = 3000,
    maxReconnectAttempts = 5,
    heartbeatInterval = 30000,
    onPoolStatus,
    onPositionUpdate,
    onPriceUpdate,
    onLiquidationWarning,
    onError,
    onConnect,
    onDisconnect,
  } = options;

  const [status, setStatus] = useState<ConnectionStatus>('disconnected');
  const [poolStatus, setPoolStatus] = useState<PoolStatusUpdate | null>(null);
  const [position, setPosition] = useState<PositionUpdate | null>(null);
  const [price, setPrice] = useState<PriceUpdate | null>(null);
  const [lastMessage, setLastMessage] = useState<WsMessage | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectAttempts = useRef(0);
  const heartbeatTimer = useRef<number | null>(null);
  const reconnectTimer = useRef<number | null>(null);

  // 메시지 처리
  const handleMessage = useCallback((event: MessageEvent) => {
    try {
      const message = JSON.parse(event.data) as WsMessage;
      setLastMessage(message);

      switch (message.type) {
        case 'PoolStatus':
          const poolData = message.data as PoolStatusUpdate;
          setPoolStatus(poolData);
          onPoolStatus?.(poolData);
          break;

        case 'PositionUpdate':
          const posData = message.data as PositionUpdate;
          setPosition(posData);
          onPositionUpdate?.(posData);
          break;

        case 'PriceUpdate':
          const priceData = message.data as PriceUpdate;
          setPrice(priceData);
          onPriceUpdate?.(priceData);
          break;

        case 'LiquidationWarning':
          const warning = message.data as LiquidationWarning;
          onLiquidationWarning?.(warning);
          break;

        case 'Pong':
          // Heartbeat 응답
          break;

        case 'Error':
          const error = message.data as { code: number; message: string };
          onError?.(new Error(error.message));
          break;
      }
    } catch (err) {
      console.error('Failed to parse WebSocket message:', err);
    }
  }, [onPoolStatus, onPositionUpdate, onPriceUpdate, onLiquidationWarning, onError]);

  // 연결
  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return;

    setStatus('connecting');

    try {
      const ws = new WebSocket(url);

      ws.onopen = () => {
        setStatus('connected');
        reconnectAttempts.current = 0;
        onConnect?.();

        // Heartbeat 시작
        if (heartbeatTimer.current) {
          clearInterval(heartbeatTimer.current);
        }
        heartbeatTimer.current = window.setInterval(() => {
          if (ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify({ action: 'Ping' }));
          }
        }, heartbeatInterval);
      };

      ws.onmessage = handleMessage;

      ws.onerror = (event) => {
        console.error('WebSocket error:', event);
        setStatus('error');
        onError?.(new Error('WebSocket connection error'));
      };

      ws.onclose = () => {
        setStatus('disconnected');
        onDisconnect?.();

        // Heartbeat 중지
        if (heartbeatTimer.current) {
          clearInterval(heartbeatTimer.current);
          heartbeatTimer.current = null;
        }

        // 재연결 시도
        if (reconnectAttempts.current < maxReconnectAttempts) {
          reconnectAttempts.current++;
          reconnectTimer.current = window.setTimeout(() => {
            connect();
          }, reconnectInterval);
        }
      };

      wsRef.current = ws;
    } catch (err) {
      setStatus('error');
      onError?.(err instanceof Error ? err : new Error('Failed to connect'));
    }
  }, [
    url,
    handleMessage,
    heartbeatInterval,
    maxReconnectAttempts,
    reconnectInterval,
    onConnect,
    onDisconnect,
    onError,
  ]);

  // 연결 해제
  const disconnect = useCallback(() => {
    if (reconnectTimer.current) {
      clearTimeout(reconnectTimer.current);
      reconnectTimer.current = null;
    }

    if (heartbeatTimer.current) {
      clearInterval(heartbeatTimer.current);
      heartbeatTimer.current = null;
    }

    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    reconnectAttempts.current = maxReconnectAttempts; // 재연결 방지
    setStatus('disconnected');
  }, [maxReconnectAttempts]);

  // 메시지 전송
  const send = useCallback((message: object) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(message));
    }
  }, []);

  // 채널 구독
  const subscribe = useCallback((channel: string) => {
    send({ action: 'Subscribe', channel });
  }, [send]);

  // 구독 취소
  const unsubscribe = useCallback((channel: string) => {
    send({ action: 'Unsubscribe', channel });
  }, [send]);

  // 자동 연결
  useEffect(() => {
    if (autoConnect) {
      connect();
    }

    return () => {
      disconnect();
    };
  }, [autoConnect, connect, disconnect]);

  return {
    // 상태
    status,
    isConnected: status === 'connected',

    // 데이터
    poolStatus,
    position,
    price,
    lastMessage,

    // 액션
    connect,
    disconnect,
    send,
    subscribe,
    unsubscribe,
  };
}

export default useWebSocket;
