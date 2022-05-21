import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import { RetentionDays } from 'aws-cdk-lib/aws-logs';

export class CdkStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    const layer = new lambda.LayerVersion(this, 'lambda-wasmtime-layer', {
      compatibleRuntimes: [
        lambda.Runtime.PROVIDED_AL2,
      ],
      compatibleArchitectures: [
        lambda.Architecture.X86_64,
      ],
      code: lambda.Code.fromAsset(
        "target/bootstrap",
        // Allowing a single file as described in
        // https://github.com/aws/aws-cdk/issues/4428#issuecomment-547931520
        { exclude: ["**", "!bootstrap"] },
      ),
      description: 'A Lambda runtime using Wasmtime to run WebAssembly workloads',
    });

    new lambda.Function(
      this,
      `lambda-wasm-example`,
      {
        description:
          'Deploying a Wasm based function on Lambda using the custom runtime',
        code: lambda.Code.fromAsset(
           "target/wasm32-wasi/release",
          { exclude: ["**", `!@(handler.wasm|handler.wat)`] }
        ),
        runtime: lambda.Runtime.PROVIDED_AL2,
        handler: "handler",
        environment: {
          RUST_BACKTRACE: 'full',
          ALLOWED_HOSTS: 'https://postman-echo.com',
          RUST_LOG: 'info',
        },
        tracing: lambda.Tracing.ACTIVE,
        layers: [layer],
        memorySize: 2048,
        timeout: cdk.Duration.seconds(10),
        // Currently, unable to use ARM64
        architecture: lambda.Architecture.X86_64,
        logRetention: RetentionDays.ONE_DAY,
      }
    )
  }
}
