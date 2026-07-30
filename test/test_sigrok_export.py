#!/usr/bin/env python3
"""Tests for Sigrok export functionality."""

import os
import shutil
import tempfile
import unittest

import sys
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'sdk'))

from agentla.capture import RawCapture
from agentla.sigrok import export_sigrok, SigrokDecoder, SigrokDecoderError


class TestSigrokExport(unittest.TestCase):

    def setUp(self):
        self.tmpdir = tempfile.mkdtemp()
        self.rc = RawCapture(
            data=b'\xAA\x55\xFF\x00\x12\x34\x56\x78',
            sample_rate=10_000_000,
            num_channels=8,
            duration=0.0004,
        )

    def tearDown(self):
        shutil.rmtree(self.tmpdir, ignore_errors=True)

    def test_export_creates_sr_directory(self):
        path = export_sigrok(self.rc, os.path.join(self.tmpdir, 'capture'))
        self.assertTrue(os.path.isdir(path))
        self.assertTrue(path.endswith('.sr'))

    def test_export_creates_metadata_file(self):
        path = export_sigrok(self.rc, os.path.join(self.tmpdir, 'capture'))
        metadata = os.path.join(path, 'metadata')
        self.assertTrue(os.path.isfile(metadata))
        with open(metadata, 'rb') as f:
            data = f.read()
        self.assertGreater(len(data), 0)

    def test_export_creates_logic_file(self):
        path = export_sigrok(self.rc, os.path.join(self.tmpdir, 'capture'))
        logic = os.path.join(path, 'logic')
        self.assertTrue(os.path.isfile(logic))
        with open(logic, 'rb') as f:
            data = f.read()
        self.assertGreater(len(data), 0)

    def test_export_only_contents_are_metadata_and_logic(self):
        path = export_sigrok(self.rc, os.path.join(self.tmpdir, 'capture'))
        contents = set(os.listdir(path))
        self.assertEqual(contents, {'metadata', 'logic'})

    def test_export_raises_on_empty_filename(self):
        with self.assertRaises(ValueError):
            export_sigrok(self.rc, '')

    def test_export_raises_on_none_filename(self):
        with self.assertRaises((ValueError, TypeError)):
            export_sigrok(self.rc, None)

    def test_export_with_existing_directory(self):
        path = os.path.join(self.tmpdir, 'capture.sr')
        os.makedirs(path)
        result = export_sigrok(self.rc, path)
        contents = set(os.listdir(result))
        self.assertEqual(contents, {'metadata', 'logic'})

    def test_export_metadata_contains_session_info(self):
        path = export_sigrok(self.rc, os.path.join(self.tmpdir, 'capture'))
        metadata_path = os.path.join(path, 'metadata')
        with open(metadata_path, 'rb') as f:
            data = f.read()
        self.assertIn(b'AGENTLA', data)
        self.assertIn(b'AgentLA', data)

    def test_export_metadata_sample_rate_is_decodable(self):
        path = export_sigrok(self.rc, os.path.join(self.tmpdir, 'capture'))
        from agentla.sigrok import _get_sr_messages
        session_msg, device_msg, ch_msg, logic_msg = _get_sr_messages()
        metadata_path = os.path.join(path, 'metadata')
        with open(metadata_path, 'rb') as f:
            data = f.read()
        session = session_msg()
        session.ParseFromString(data)
        device = session.devices[0]
        self.assertEqual(device.sample_rate, 10000000)

    def test_export_logic_contains_sample_data(self):
        path = export_sigrok(self.rc, os.path.join(self.tmpdir, 'capture'))
        logic_path = os.path.join(path, 'logic')
        with open(logic_path, 'rb') as f:
            data = f.read()
        self.assertIn(b'\xAA', data)
        self.assertIn(b'\x55', data)

    def test_export_with_different_sample_rates(self):
        for rate in [100_000, 1_000_000, 10_000_000, 100_000_000]:
            rc = RawCapture(
                data=b'\x00\xFF',
                sample_rate=rate,
                num_channels=8,
                duration=0.0001,
            )
            path = export_sigrok(rc, os.path.join(self.tmpdir, f'capture_{rate}'))
            self.assertTrue(os.path.isdir(path))


class TestSigrokDecoderImport(unittest.TestCase):

    def test_sigrok_decoder_raises_when_sigrokdecode_missing(self):
        with self.assertRaises(SigrokDecoderError):
            SigrokDecoder('I2C')


if __name__ == '__main__':
    unittest.main()