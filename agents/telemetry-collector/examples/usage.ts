/**
 * Usage examples for Telemetry Collector Agent
 * Copyright 2025 LLM Observatory Contributors
 * SPDX-License-Identifier: Apache-2.0
 */

// Example 1: Single telemetry event
const singleEvent = {
  provider: 'openai',
  model: 'gpt-4',
  inputType: 'chat',
  input: [
    { role: 'system', content: 'You are a helpful assistant.' },
    { role: 'user', content: 'What is the capital of France?' },
  ],
  output: {
    role: 'assistant',
    content: 'The capital of France is Paris.',
  },
  tokenUsage: {
    promptTokens: 25,
    completionTokens: 10,
    totalTokens: 35,
  },
  cost: {
    amountUsd: 0.00105,
    currency: 'USD',
    promptCost: 0.00075,
    completionCost: 0.0003,
  },
  latency: {
    totalMs: 1250,
    ttftMs: 450,
  },
  metadata: {
    userId: 'user-12345',
    sessionId: 'session-abc',
    environment: 'production',
    tags: ['customer-support', 'faq'],
  },
  serviceName: 'support-chatbot',
  serviceVersion: '2.1.0',
};

// Example 2: Batch telemetry events
const batchEvents = [
  {
    provider: 'anthropic',
    model: 'claude-3-opus',
    inputType: 'text',
    input: 'Write a haiku about coding',
    output: 'Code flows like water\nBugs hide in silent shadows\nTests bring clarity',
    tokenUsage: {
      promptTokens: 8,
      completionTokens: 15,
      totalTokens: 23,
    },
    cost: {
      amountUsd: 0.00069,
    },
    metadata: {
      environment: 'staging',
    },
  },
  {
    provider: 'google',
    model: 'gemini-pro',
    inputType: 'multimodal',
    input: {
      text: 'Describe this image',
      image: 'base64_encoded_image_data',
    },
    tokenUsage: {
      promptTokens: 258,
      completionTokens: 45,
      totalTokens: 303,
    },
  },
];

// Example 3: Sending to deployed function
async function sendTelemetry() {
  const functionUrl = 'https://us-central1-project.cloudfunctions.net/telemetry-collector';

  // Single event
  const response1 = await fetch(`${functionUrl}/ingest`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(singleEvent),
  });

  const result1 = await response1.json();
  console.log('Single event result:', result1);

  // Batch events
  const response2 = await fetch(`${functionUrl}/ingest`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(batchEvents),
  });

  const result2 = await response2.json();
  console.log('Batch result:', result2);

  // Health check
  const health = await fetch(`${functionUrl}/health`);
  const healthData = await health.json();
  console.log('Health status:', healthData);
}

// Example 4: Error handling
async function sendWithErrorHandling() {
  const functionUrl = 'https://us-central1-project.cloudfunctions.net/telemetry-collector';

  try {
    const response = await fetch(`${functionUrl}/ingest`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(singleEvent),
    });

    if (!response.ok) {
      const error = await response.json();
      console.error('Ingestion failed:', error);
      return;
    }

    const result = await response.json();
    console.log('Success:', result);
    console.log('Execution ref:', result.executionRef);
    console.log('Processing time:', result.processingTimeMs, 'ms');
  } catch (error) {
    console.error('Network error:', error);
  }
}

// Example 5: TypeScript SDK usage (if integrated with client SDK)
import { telemetryCollector } from '@observatory/telemetry-collector';

async function handleRequest(request: Request): Promise<Response> {
  return await telemetryCollector(request);
}

// Example 6: Testing locally with Node.js http server
import { createServer } from 'http';

const PORT = 8080;

const server = createServer(async (req, res) => {
  // Convert Node.js request to Fetch API Request
  const url = `http://localhost:${PORT}${req.url}`;
  const body = req.method === 'POST' ? await readBody(req) : undefined;

  const request = new Request(url, {
    method: req.method,
    headers: req.headers as HeadersInit,
    body,
  });

  // Handle with telemetry collector
  const response = await telemetryCollector(request);

  // Convert Fetch API Response to Node.js response
  res.statusCode = response.status;
  response.headers.forEach((value, key) => {
    res.setHeader(key, value);
  });

  const responseBody = await response.text();
  res.end(responseBody);
});

function readBody(req: any): Promise<string> {
  return new Promise((resolve) => {
    let body = '';
    req.on('data', (chunk: Buffer) => {
      body += chunk.toString();
    });
    req.on('end', () => {
      resolve(body);
    });
  });
}

server.listen(PORT, () => {
  console.log(`Telemetry Collector running at http://localhost:${PORT}`);
  console.log(`Health: http://localhost:${PORT}/health`);
  console.log(`Ingest: http://localhost:${PORT}/ingest`);
});

// Export examples
export { singleEvent, batchEvents, sendTelemetry, sendWithErrorHandling };
