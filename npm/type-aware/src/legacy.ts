export type CorsaExecutableConfig = {
  executable: string;
  rootUri?: string;
  initializationOptions?: unknown;
};

export type CorsaSnapshot = {
  uri: string;
  text: string;
  version?: number;
};

export type CorsaTypeAwareSession = {
  updateSnapshot(snapshot: CorsaSnapshot): Promise<unknown>;
  callJson(method: string, params?: unknown): Promise<unknown>;
  close(): Promise<void>;
};

type CorsaClient = {
  initializeAsync?: (params?: unknown) => Promise<unknown>;
  updateSnapshotAsync?: (snapshot: CorsaSnapshot) => Promise<unknown>;
  callJsonAsync?: (method: string, params?: unknown) => Promise<unknown>;
  closeAsync?: () => Promise<unknown>;
};

export async function createCorsaTypeAwareSession(
  config: CorsaExecutableConfig,
): Promise<CorsaTypeAwareSession> {
  if (!config.executable) {
    throw new Error('Corsa executable path is required for type-aware rules.');
  }

  const corsa = (await import('@corsa-bind/napi')) as {
    CorsaApiClient?: {
      spawnAsync?: (options: { executable: string }) => Promise<CorsaClient>;
      spawn?: (options: { executable: string }) => CorsaClient;
    };
  };
  const clientFactory = corsa.CorsaApiClient;

  if (!clientFactory) {
    throw new Error('@corsa-bind/napi did not expose CorsaApiClient.');
  }

  const client =
    (await clientFactory.spawnAsync?.({ executable: config.executable })) ??
    clientFactory.spawn?.({ executable: config.executable });

  if (!client) {
    throw new Error('Failed to spawn Corsa API client.');
  }

  await client.initializeAsync?.({
    rootUri: config.rootUri,
    initializationOptions: config.initializationOptions,
  });

  return {
    updateSnapshot(snapshot) {
      if (!client.updateSnapshotAsync) {
        throw new Error('Corsa client does not support updateSnapshotAsync.');
      }
      return client.updateSnapshotAsync(snapshot);
    },
    callJson(method, params) {
      if (!client.callJsonAsync) {
        throw new Error('Corsa client does not support callJsonAsync.');
      }
      return client.callJsonAsync(method, params);
    },
    async close() {
      await client.closeAsync?.();
    },
  };
}

export function pathToFileUri(path: string): string {
  const normalized = path.replace(/\\/g, '/');
  const prefixed = normalized.startsWith('/') ? normalized : `/${normalized}`;
  return `file://${encodeURI(prefixed)}`;
}
