#!/usr/bin/env python3
"""Stage verified native installers without upgrading their engineering evidence."""
from __future__ import annotations

import argparse
import importlib.util
import json
import os
import re
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent.parent
MAX_METADATA_BYTES = 2 * 1024 * 1024


def load_script(name):
    spec = importlib.util.spec_from_file_location(name.replace('-', '_'), ROOT / 'scripts' / (name + '.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


packaging = load_script('package-installers')


def regular_file(folder, name, *, metadata=False):
    path = folder / name
    if Path(name).name != name or path.is_symlink() or not path.is_file() or path.stat().st_size == 0:
        raise ValueError('Missing or unsafe release input: ' + name)
    if not path.resolve().is_relative_to(folder.resolve()):
        raise ValueError('Release input escapes its artifact directory: ' + name)
    if metadata and path.stat().st_size > MAX_METADATA_BYTES:
        raise ValueError('Release metadata exceeds its bound: ' + name)
    return path


def read_object(folder, name):
    data = json.loads(regular_file(folder, name, metadata=True).read_text(encoding='utf-8'))
    if not isinstance(data, dict):
        raise ValueError('Release metadata must be an object: ' + name)
    return data


def validate_acceptance(report, version, fingerprint, required_scope):
    acceptance = report.get('ux_acceptance')
    if (report.get('mode') != 'release' or report.get('ok') is not True or report.get('errors') != []
            or report.get('release_ready') is not True or report.get('ux_gate_status') != 'accepted'
            or report.get('ux_open_blockers') != [] or not isinstance(acceptance, dict)
            or not acceptance.get('reviewer') or not acceptance.get('review')):
        raise ValueError('Current independent named-scope release acceptance is required')
    if report.get('version') != version or report.get('source_fingerprint') != fingerprint:
        raise ValueError('Release acceptance must match the exact version and source fingerprint')
    if report.get('required_ux_scope') != required_scope or acceptance.get('scope') != required_scope:
        raise ValueError('Release acceptance must match the selected scope')
    return acceptance


def stage_assets(downloads, output, version, commit, fingerprint, required_scope, tag, release_report, notes):
    """Validate everything before publishing a new owned staging directory."""
    packaging.windows_version(version)
    if '-' not in version or tag != 'v' + version:
        raise ValueError('An existing matching prerelease tag is required')
    if not re.fullmatch(r'[a-f0-9]{40}', commit) or not re.fullmatch(r'[a-f0-9]{64}', fingerprint):
        raise ValueError('Exact source commit and fingerprint are required')
    if not isinstance(required_scope, str) or not required_scope:
        raise ValueError('A selected release scope is required')
    if output.exists():
        raise ValueError('Release staging output exists; use a fresh destination')
    if downloads.is_symlink() or not downloads.is_dir():
        raise ValueError('Installer artifact directory is missing or unsafe')
    acceptance = validate_acceptance(release_report, version, fingerprint, required_scope)
    if not notes.strip() or len(notes.encode('utf-8')) > MAX_METADATA_BYTES:
        raise ValueError('Versioned release notes are missing or oversized')
    manifest_path = regular_file(downloads, 'distribution-manifest.json', metadata=True)
    manifest = read_object(downloads, manifest_path.name)
    if (manifest.get('schema_version') != 1 or manifest.get('kind') != 'engineering-distribution-candidate'
            or manifest.get('release_ready') is not False or not isinstance(manifest.get('platforms'), dict)
            or set(manifest['platforms']) != set(packaging.TARGETS)):
        raise ValueError('Combined engineering distribution identity is invalid')
    expected_files = {'distribution-manifest.json', 'SHA256SUMS', 'INSTALL.md'}
    packages = {}
    # Hardlinks stay on the input filesystem and avoid duplicating large packages just
    # to reuse the existing per-platform validator. They are never part of output.
    with tempfile.TemporaryDirectory(prefix='.writercraft-verify-', dir=downloads.parent) as folder:
        verification = Path(folder)
        for platform_name in sorted(packaging.TARGETS):
            evidence_name = platform_name + '-evidence.json'
            original = regular_file(downloads, evidence_name, metadata=True)
            evidence = read_object(downloads, evidence_name)
            if evidence != manifest['platforms'][platform_name]:
                raise ValueError('Combined manifest differs from platform evidence: ' + platform_name)
            if (evidence.get('platform') != platform_name or evidence.get('version') != version
                    or evidence.get('commit') != commit or evidence.get('source_fingerprint') != fingerprint):
                raise ValueError('Every installer must match the exact candidate source and version')
            expected_name = 'WriterCraft-' + version + '-' + platform_name + packaging.TARGETS[platform_name][3]
            package = regular_file(downloads, expected_name)
            if evidence.get('package', {}).get('filename') != expected_name:
                raise ValueError('Installer filename differs from expected platform identity')
            expected_files.update({evidence_name, expected_name})
            packages[platform_name] = (package, original, evidence)
            destination = verification / platform_name
            destination.mkdir()
            os.link(package, destination / expected_name)
            shutil.copyfile(original, destination / 'distribution-evidence.json')
            (destination / 'SHA256SUMS').write_text(
                str(evidence['package'].get('sha256')) + '  ' + expected_name + '\n', encoding='utf-8')
        packaging.validate_candidates(verification)
    sums = '\n'.join(sorted(evidence['package']['sha256'] + '  ' + package.name
                            for package, _, evidence in packages.values())) + '\n'
    if regular_file(downloads, 'SHA256SUMS', metadata=True).read_text(encoding='utf-8') != sums:
        raise ValueError('Combined checksum file differs from verified installer evidence')
    regular_file(downloads, 'INSTALL.md', metadata=True)
    if {path.name for path in downloads.iterdir()} != expected_files:
        raise ValueError('Installer artifact contains missing or unexpected release files')
    output.mkdir(parents=True)
    try:
        for name in sorted(expected_files):
            shutil.copyfile(downloads / name, output / name)
        write_json(output / 'release-evidence.json', release_report)
        write_json(output / 'release-manifest.json', {
            'schema_version': 1,
            'kind': 'scoped-prerelease-promotion',
            'version': version,
            'tag': tag,
            'commit': commit,
            'source_fingerprint': fingerprint,
            'prerelease': True,
            'independent_acceptance': acceptance,
            'full_editor_parity': False,
            'engineering_distribution_manifest_sha256': packaging.sha256(manifest_path),
            'platforms': {name: {
                'target': evidence['target'],
                'package': evidence['package'],
                'engineering_evidence_sha256': packaging.sha256(original),
                'platform_ux_qualified': evidence['platform_ux_qualified'],
            } for name, (_, original, evidence) in sorted(packages.items())},
        })
        (output / 'release-notes.md').write_text(notes.rstrip() + '''

### Installers

- **macOS Apple silicon (14+):** open the `.dmg` and drag WriterCraft into Applications.
- **Windows x64:** run the per-user `-setup.exe` installer.
- **Debian 12 / Ubuntu 24.04 amd64:** install the `.deb` with `sudo apt install ./<filename>.deb`.

See the attached `INSTALL.md` for data locations, checksums and installation details.
The macOS application is ad-hoc signed, without Developer ID notarization. The Windows
installer is unsigned. Platform-wide native interaction and complete original-product
parity remain unqualified. `release-manifest.json` records the independently reviewed
scope; original engineering installer evidence is preserved unchanged.
''', encoding='utf-8')
        # Recheck after copying, so a successful stage describes the bytes to upload.
        for package, _, evidence in packages.values():
            if packaging.sha256(output / package.name) != evidence['package']['sha256']:
                raise ValueError('Staged installer changed during copy')
    except BaseException:
        shutil.rmtree(output)
        raise
    return output


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--downloads', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--commit')
    parser.add_argument('--tag')
    parser.add_argument('--release-evidence', type=Path)
    parser.add_argument('--notes', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args(argv)
    if args.self_test:
        return 0 if unittest.TextTestRunner(verbosity=2).run(
            unittest.defaultTestLoader.loadTestsFromTestCase(InstallerReleaseContracts)).wasSuccessful() else 1
    if not all([args.downloads, args.output, args.commit, args.tag, args.release_evidence, args.notes]):
        parser.error('staging requires downloads, output, commit, tag, release-evidence and notes')
    release = load_script('release-check')
    metadata = release.workspace_metadata()
    report_path = args.release_evidence.resolve(strict=True)
    notes_path = args.notes.resolve(strict=True)
    for path in [report_path, notes_path]:
        if path.stat().st_size > MAX_METADATA_BYTES:
            raise ValueError('Release decision or notes exceed their bound')
    report = json.loads(report_path.read_text(encoding='utf-8'))
    if not isinstance(report, dict):
        raise ValueError('Release decision must be an object')
    stage_assets(args.downloads.resolve(strict=True), args.output.resolve(), metadata['version'],
                 args.commit, release.source_fingerprint(), metadata['required_ux_scope'], args.tag,
                 report, notes_path.read_text(encoding='utf-8'))
    return 0


class InstallerReleaseContracts(unittest.TestCase):
    version = '0.1.0-alpha.2'
    commit = 'a' * 40
    fingerprint = 'b' * 64
    scope = 'native-rich-editor-alpha'

    def fixture(self, root):
        source = root / 'candidates'
        source.mkdir()
        for platform_name, (_, _, target, extension) in packaging.TARGETS.items():
            folder = source / platform_name
            folder.mkdir()
            package = folder / ('WriterCraft-' + self.version + '-' + platform_name + extension)
            package.write_bytes(b'Synthetic installer ' + platform_name.encode())
            evidence = {'schema_version': 1, 'kind': 'engineering-distribution-candidate',
                        'version': self.version, 'commit': self.commit, 'source_fingerprint': self.fingerprint,
                        'platform': platform_name, 'target': target, 'cargo_features': ['editor-preview'],
                        'application_id': packaging.APP_ID, 'release_ready': False, 'platform_ux_qualified': False,
                        'checks': dict.fromkeys(packaging.REQUIRED_CHECKS[platform_name], True),
                        'package': {'filename': package.name, 'sha256': packaging.sha256(package),
                                    'size': package.stat().st_size}}
            if platform_name == 'linux-amd64':
                evidence['linux_install_tests'] = {name: {'passed': True} for name in ['debian-12', 'ubuntu-24.04']}
            write_json(folder / 'distribution-evidence.json', evidence)
            (folder / 'SHA256SUMS').write_text(evidence['package']['sha256'] + '  ' + package.name + '\n')
        combined = root / 'combined'
        packaging.collect(source, combined)
        report = {'ok': True, 'mode': 'release', 'version': self.version,
                  'source_fingerprint': self.fingerprint, 'required_ux_scope': self.scope,
                  'release_ready': True, 'errors': [], 'ux_gate_status': 'accepted', 'ux_open_blockers': [],
                  'ux_acceptance': {'scope': self.scope, 'reviewer': 'Independent fixture reviewer',
                                    'review': 'docs/design/UX_GATE.json'}}
        return combined, report

    def stage(self, root, combined, report, **overrides):
        args = dict(downloads=combined, output=root / 'release', version=self.version,
                    commit=self.commit, fingerprint=self.fingerprint, required_scope=self.scope,
                    tag='v' + self.version, release_report=report, notes='## [' + self.version + ']\n\nSynthetic alpha fixture.\n')
        args.update(overrides)
        return stage_assets(**args)

    def test_three_installers_are_staged_without_upgrading_engineering_evidence(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            combined, report = self.fixture(root)
            before = {path.name: path.read_bytes() for path in combined.iterdir()}
            output = self.stage(root, combined, report)
            for name, data in before.items():
                self.assertEqual((output / name).read_bytes(), data)
            promoted = json.loads((output / 'release-manifest.json').read_text())
            self.assertEqual(promoted['independent_acceptance']['scope'], self.scope)
            self.assertFalse(promoted['full_editor_parity'])
            self.assertTrue(all(not item['platform_ux_qualified'] for item in promoted['platforms'].values()))
            self.assertFalse(json.loads((output / 'distribution-manifest.json').read_text())['release_ready'])
            self.assertIn('.dmg', (output / 'release-notes.md').read_text())

    def test_tampering_missing_platform_unexpected_file_and_symlink_are_refused_before_stage(self):
        for mutation in ['package', 'missing', 'unexpected', 'symlink']:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                combined, report = self.fixture(root)
                package = next(combined.glob('*.dmg'))
                if mutation == 'package':
                    package.write_bytes(b'Tampered installer')
                elif mutation == 'missing':
                    package.unlink()
                elif mutation == 'unexpected':
                    (combined / 'private.sqlite').write_bytes(b'Never publish this fixture')
                else:
                    actual = root / 'outside.dmg'
                    package.rename(actual)
                    package.symlink_to(actual)
                with self.assertRaises(ValueError):
                    self.stage(root, combined, report)
                self.assertFalse((root / 'release').exists())

    def test_wrong_source_version_fingerprint_scope_and_tag_are_refused(self):
        for field, value in [('commit', 'c' * 40), ('fingerprint', 'd' * 64),
                             ('version', '0.1.0-alpha.3'), ('required_scope', 'full-editor-parity'),
                             ('tag', 'v0.1.0-alpha.1')]:
            with self.subTest(field=field), tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                combined, report = self.fixture(root)
                with self.assertRaises(ValueError):
                    self.stage(root, combined, report, **{field: value})
                self.assertFalse((root / 'release').exists())

    def test_pending_or_stale_acceptance_cannot_be_promoted_by_installer_success(self):
        for field, value in [('ok', False), ('mode', 'source'), ('release_ready', False),
                             ('ux_gate_status', 'pending'), ('ux_open_blockers', ['Unresolved blocker']),
                             ('source_fingerprint', 'c' * 64), ('ux_acceptance', {})]:
            with self.subTest(field=field), tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                combined, report = self.fixture(root)
                report[field] = value
                with self.assertRaises(ValueError):
                    self.stage(root, combined, report)
                self.assertFalse((root / 'release').exists())

    def test_manifest_disagreement_and_unearned_engineering_claim_are_refused(self):
        for mutation in ['manifest', 'release_claim', 'checks', 'linux_baseline', 'feature']:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                combined, report = self.fixture(root)
                evidence_path = combined / 'linux-amd64-evidence.json'
                evidence = json.loads(evidence_path.read_text())
                if mutation == 'release_claim':
                    evidence['release_ready'] = True
                elif mutation == 'checks':
                    evidence['checks'] = {}
                elif mutation == 'linux_baseline':
                    evidence['linux_install_tests'] = {}
                elif mutation == 'feature':
                    evidence['cargo_features'] = []
                else:
                    evidence['commit'] = 'c' * 40
                write_json(evidence_path, evidence)
                if mutation != 'manifest':
                    manifest = read_object(combined, 'distribution-manifest.json')
                    manifest['platforms']['linux-amd64'] = evidence
                    write_json(combined / 'distribution-manifest.json', manifest)
                with self.assertRaises(ValueError):
                    self.stage(root, combined, report)
                self.assertFalse((root / 'release').exists())

    def test_combined_checksums_and_existing_output_are_not_silently_replaced(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            combined, report = self.fixture(root)
            sums = combined / 'SHA256SUMS'
            original = sums.read_bytes()
            sums.write_text('0' * 64 + '  wrong.dmg\n')
            with self.assertRaisesRegex(ValueError, 'checksum'):
                self.stage(root, combined, report)
            self.assertFalse((root / 'release').exists())
            sums.write_bytes(original)
            output = root / 'release'
            output.mkdir()
            sentinel = output / 'keep.txt'
            sentinel.write_text('Existing release data')
            with self.assertRaisesRegex(ValueError, 'exists'):
                self.stage(root, combined, report)
            self.assertEqual(sentinel.read_text(), 'Existing release data')


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        print('Installer release staging failed: ' + str(error), file=sys.stderr)
        sys.exit(1)
