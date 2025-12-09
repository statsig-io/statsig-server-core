// Mimics what SCRAPI does.
// See MapProtobufSpecs.ts in the statsig repo: https://github.com/statsig-io/statsig/pull/92401
//
import { getRootedPath } from '@/utils/file_utils.js';
import { Log } from '@/utils/terminal_utils.js';
import crypto from 'node:crypto';
import fs from 'node:fs';
import { promisify } from 'node:util';
import { brotliCompress } from 'node:zlib';
import pb from 'protobufjs';

import { CommandBase } from './command_base.js';

const brotliCompressAsync = promisify(brotliCompress);

export class SyncTestData extends CommandBase {
  constructor() {
    super(import.meta.url);

    this.description('Make test proto data');
  }

  override async run() {
    Log.stepBegin('Loading schema');

    const root = new pb.Root();

    const schema = await root.load(
      getRootedPath('api-interface-definitions/protos/config_specs.proto'),
      { keepCase: true },
    );

    const SpecsEnvelope = schema.lookupType(
      'statsig_config_specs.SpecsEnvelope',
    );
    addUnexpectedField(
      SpecsEnvelope,
      'an_unexpected_envelope_field_for_testing',
      1234,
    );

    const SpecsTopLevel = schema.lookupType(
      'statsig_config_specs.SpecsTopLevel',
    );
    addUnexpectedField(
      SpecsTopLevel,
      'an_unexpected_top_level_field_for_testing',
      1234,
    );

    const Spec = schema.lookupType('statsig_config_specs.Spec');
    addUnexpectedField(Spec, 'an_unexpected_spec_field_for_testing', 1234);

    const messages: Uint8Array[] = [];

    // ------------ TOP LEVEL ------------

    const topLevelData = {
      has_updates: true,
      time: 1234567890,
      company_id: 'test_company_id',
      response_format: 'test_response_format',
      checksum: 'test_checksum',
      condition_map: {},
      rest: Buffer.from(
        JSON.stringify({
          experiment_to_layer: {},
        }),
      ),
      an_unexpected_top_level_field_for_testing: 'some-unexpected-field',
    };

    messages.push(
      encodeEnvelope(SpecsEnvelope, {
        kind: 2,
        name: 'TOP_LEVEL',
        checksum: 'test_checksum',
        data: SpecsTopLevel.encode(topLevelData).finish(),
        an_unexpected_envelope_field_for_testing: 'some-unexpected-field',
      }),
    );

    messages.push(
      encodeEnvelope(SpecsEnvelope, {
        kind: 2,
        name: 'TOP_LEVEL',
        checksum: 'test_checksum',
        data: SpecsTopLevel.encode(topLevelData).finish(),
        an_unexpected_envelope_field_for_testing: 'some-unexpected-field',
      }),
    );

    Log.stepProgress('Added top level envelope');

    // ------------ FEATURE GATE WITHOUT UNEXPECTED FIELD ------------

    const gateWithoutUnexpectedField = {
      type: 'feature_gate',
      salt: 'test_salt_for_gate_without_unexpected_field',
      enabled: true,
      default_value: { bool_value: true },
      rules: [],
      id_type: { known_id_type: 1 }, // IdType.USER_ID
      entity: 1, // EntityType.FEATURE_GATE
      version: 8,
    };

    messages.push(
      makeFeatureGateEnvelope(
        SpecsEnvelope,
        Spec,
        'gate_without_unexpected_field',
        gateWithoutUnexpectedField,
      ),
    );

    Log.stepProgress('Added feature gate envelope');

    // ------------ FEATURE GATE WITH UNEXPECTED FIELD ------------

    const gateWithUnexpectedField = {
      type: 'feature_gate',
      salt: '1e85dcdc-af1a-484b-bb3b-b2e04fb4e658',
      enabled: true,
      default_value: { bool_value: true },
      rules: [],
      id_type: { known_id_type: 1 }, // IdType.USER_ID
      entity: 99999, // Some Unknown EntityType
      version: 8,
      an_unexpected_spec_field_for_testing: 'some-unexpected-field',
    };

    messages.push(
      makeFeatureGateEnvelope(
        SpecsEnvelope,
        Spec,
        'gate_with_unexpected_field',
        gateWithUnexpectedField,
      ),
    );

    Log.stepProgress('Added unsupported feature gate envelope');

    // ------------ UNKNOWN ENVELOPE KIND ------------

    const unknownEnvelopeKind = SpecsEnvelope.create({
      kind: 999999, // invalid enum value
      name: 'UNKNOWN_ENVELOPE_KIND',
      checksum: 'test_checksum',
      data: Buffer.from([]),
    });

    messages.push(SpecsEnvelope.encodeDelimited(unknownEnvelopeKind).finish());

    Log.stepProgress('Added unknown envelope kind envelope');

    // ------------ DONE ------------

    const doneEnvelope = SpecsEnvelope.create({
      kind: 1,
      name: 'DONE',
      checksum: 'test_checksum',
      data: Buffer.from([]),
    });

    messages.push(SpecsEnvelope.encodeDelimited(doneEnvelope).finish());

    Log.stepProgress('Added done envelope');

    const buffer = Buffer.concat(
      messages.map((m) => Buffer.from(m.buffer, m.byteOffset, m.byteLength)),
    );

    const compressed = await brotliCompressAsync(buffer);
    Log.stepProgress('Compressed data');

    const outputPath = getRootedPath(
      'statsig-rust/tests/data/unknown_enum.pb.br',
    );
    fs.writeFileSync(outputPath, compressed);

    Log.stepProgress('Wrote compressed data to file: ' + outputPath);

    Log.stepEnd('Finished making test proto data');
  }
}

function makeMd5Checksum(data: Uint8Array) {
  return crypto.createHash('md5').update(data).digest('base64');
}

function addUnexpectedField(type: pb.Type, name: string, id: number) {
  type.add(new pb.Field(name, id, 'string'));
}

function makeFeatureGateEnvelope(
  SpecsEnvelope: pb.Type,
  Spec: pb.Type,
  name: string,
  data: Record<string, any>,
) {
  const fgData = Spec.encode(data).finish();

  const fgEnvelope = SpecsEnvelope.create({
    kind: 3,
    name,
    checksum: makeMd5Checksum(fgData),
    data: fgData,
  });

  return SpecsEnvelope.encodeDelimited(fgEnvelope).finish();
}

function encodeEnvelope(SpecsEnvelope: pb.Type, data: Record<string, any>) {
  const envelope = SpecsEnvelope.create(data);
  const encoded = SpecsEnvelope.encodeDelimited(envelope).finish();
  const decoded = SpecsEnvelope.decodeDelimited(encoded);

  console.log(`-------------------------------- [${data.name}]`);
  console.log('Envelope:', envelope);
  console.log('Encoded:', encoded);
  console.log('Decoded:', decoded);

  return encoded;
}
