import type { TunnelInfo } from '../../rust-api/types';

export type OpenUpdateInstanceModalData = {
  instanceId: number;
  url: string;
};

export type OpenUpdateTunnelModalData = TunnelInfo;
