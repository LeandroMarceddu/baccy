import { writable } from 'svelte/store';

export interface IpConfig {
  type: 'ip';
  ip: string;
  port: number;
  bbmdEnabled?: boolean;
  bbmdAddress?: string;
  bbmdPort?: number;
  bbmdTtl?: number;
}

export interface MstpConfig {
  type: 'mstp';
  portName: string;
  baudRate: number;
  localMac: number;
}

export type TransportConfig = IpConfig | MstpConfig;

export interface TransportState {
  type: 'ip' | 'mstp' | null;
  config: TransportConfig | null;
  connected: boolean;
}

export const transportState = writable<TransportState>({
  type: null,
  config: null,
  connected: false
});
