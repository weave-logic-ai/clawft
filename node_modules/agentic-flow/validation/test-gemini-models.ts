#!/usr/bin/env tsx

/**
 * Test Gemini proxy with multiple models
 * Validates issue #55 fix across different Gemini model versions
 */

import Anthropic from '@anthropic-ai/sdk';

const GEMINI_PROXY_URL = process.env.GEMINI_PROXY_URL || 'http://localhost:3001';
const GOOGLE_GEMINI_API_KEY = process.env.GOOGLE_GEMINI_API_KEY;

if (!GOOGLE_GEMINI_API_KEY) {
  console.error('❌ GOOGLE_GEMINI_API_KEY not set in environment');
  process.exit(1);
}

// Gemini models to test
const GEMINI_MODELS = [
  'gemini-2.0-flash-exp',
  'gemini-1.5-pro',
  'gemini-1.5-flash',
  'gemini-1.5-flash-8b',
];

// Test tool with exclusiveMinimum/exclusiveMaximum (like Claude Code uses)
const testTool: Anthropic.Tool = {
  name: 'get_weather',
  description: 'Get weather information for a location',
  input_schema: {
    type: 'object',
    properties: {
      location: {
        type: 'string',
        description: 'City name'
      },
      temperature_min: {
        type: 'number',
        exclusiveMinimum: -100,
        exclusiveMaximum: 100,
        description: 'Minimum temperature in Celsius'
      },
      days: {
        type: 'integer',
        exclusiveMinimum: 0,
        description: 'Number of forecast days'
      }
    },
    required: ['location']
  }
};

interface TestResult {
  model: string;
  success: boolean;
  responseTime: number;
  error?: string;
  responseId?: string;
}

async function testModel(model: string): Promise<TestResult> {
  const startTime = Date.now();

  try {
    const client = new Anthropic({
      apiKey: GOOGLE_GEMINI_API_KEY,
      baseURL: GEMINI_PROXY_URL
    });

    const response = await client.messages.create({
      model: model,
      max_tokens: 512,
      messages: [
        {
          role: 'user',
          content: 'What is the weather like today? Just give a brief response.'
        }
      ],
      tools: [testTool]
    });

    const responseTime = Date.now() - startTime;

    return {
      model,
      success: true,
      responseTime,
      responseId: response.id
    };

  } catch (error: any) {
    const responseTime = Date.now() - startTime;

    // Check if error is about exclusiveMinimum
    const isSchemaError = error.message?.includes('exclusiveMinimum') ||
                         error.message?.includes('exclusiveMaximum');

    return {
      model,
      success: false,
      responseTime,
      error: isSchemaError ? 'SCHEMA ERROR (exclusiveMinimum/Maximum)' : error.message
    };
  }
}

async function main() {
  console.log('═══════════════════════════════════════════════════════════');
  console.log('  Gemini Models Multi-Model Validation');
  console.log('  Testing exclusiveMinimum/Maximum fix across models');
  console.log('═══════════════════════════════════════════════════════════\n');

  console.log(`Proxy URL: ${GEMINI_PROXY_URL}`);
  console.log(`API Key: ${GOOGLE_GEMINI_API_KEY.substring(0, 10)}...\n`);

  console.log('📋 Test Tool Schema (includes exclusiveMinimum/Maximum):');
  console.log(JSON.stringify(testTool.input_schema, null, 2));
  console.log('\n');

  const results: TestResult[] = [];

  console.log('🚀 Testing Gemini models...\n');

  for (const model of GEMINI_MODELS) {
    process.stdout.write(`Testing ${model.padEnd(25)} ... `);

    const result = await testModel(model);
    results.push(result);

    if (result.success) {
      console.log(`✅ PASS (${result.responseTime}ms)`);
    } else {
      console.log(`❌ FAIL - ${result.error}`);
    }
  }

  console.log('\n═══════════════════════════════════════════════════════════');
  console.log('  TEST RESULTS SUMMARY');
  console.log('═══════════════════════════════════════════════════════════\n');

  const successCount = results.filter(r => r.success).length;
  const failCount = results.filter(r => r.success === false).length;
  const schemaErrorCount = results.filter(r => r.error?.includes('SCHEMA ERROR')).length;

  console.log('📊 Overall Statistics:');
  console.log(`  Total Models Tested: ${results.length}`);
  console.log(`  Successful: ${successCount} ✅`);
  console.log(`  Failed: ${failCount} ❌`);
  console.log(`  Schema Errors: ${schemaErrorCount} 🐛\n`);

  console.log('📋 Detailed Results:\n');

  for (const result of results) {
    console.log(`Model: ${result.model}`);
    console.log(`  Status: ${result.success ? '✅ PASS' : '❌ FAIL'}`);
    console.log(`  Response Time: ${result.responseTime}ms`);
    if (result.responseId) {
      console.log(`  Response ID: ${result.responseId}`);
    }
    if (result.error) {
      console.log(`  Error: ${result.error}`);
    }
    console.log('');
  }

  console.log('═══════════════════════════════════════════════════════════');

  if (successCount === results.length) {
    console.log('✅ ALL MODELS PASSED - Fix working across all Gemini models!');
    console.log('═══════════════════════════════════════════════════════════\n');

    console.log('🎉 Success Metrics:');
    console.log(`  - All ${results.length} models tested successfully`);
    console.log('  - No exclusiveMinimum/Maximum errors detected');
    console.log('  - Tool schemas properly cleaned for Gemini API');
    console.log('  - Issue #55 fix validated across all model versions\n');

    const avgResponseTime = results.reduce((sum, r) => sum + r.responseTime, 0) / results.length;
    console.log(`Average Response Time: ${avgResponseTime.toFixed(0)}ms\n`);

    process.exit(0);
  } else if (schemaErrorCount > 0) {
    console.log('❌ SCHEMA ERRORS DETECTED - Fix not working correctly!');
    console.log('═══════════════════════════════════════════════════════════\n');
    console.log('⚠️  Some models still rejecting exclusiveMinimum/Maximum');
    console.log('   This indicates the cleanSchema fix needs improvement.\n');
    process.exit(1);
  } else {
    console.log('⚠️  SOME TESTS FAILED - Check errors above');
    console.log('═══════════════════════════════════════════════════════════\n');
    console.log(`${successCount}/${results.length} models passed`);
    console.log('Errors may be related to API keys, rate limits, or model availability.\n');
    process.exit(failCount > 0 ? 1 : 0);
  }
}

main().catch(err => {
  console.error('Fatal error:', err);
  process.exit(1);
});
