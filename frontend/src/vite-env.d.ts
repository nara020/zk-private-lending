/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_URL: string;
  readonly VITE_LENDING_POOL_ADDRESS: string;
  readonly VITE_USDC_ADDRESS: string;
  readonly VITE_CHAIN_ID: string;
  readonly VITE_NETWORK_NAME: string;
  readonly VITE_RPC_URL: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
