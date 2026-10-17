"""No signing, account access, network or OS mutation: exercise the submission payload gate."""
import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
import release

class PreparedPayloadTests(unittest.TestCase):
    def test_payload_changes_require_a_new_approval(self):
        with tempfile.TemporaryDirectory() as root:
            root=Path(root);app=root/'FlowGuard.app';app.mkdir();(app/'binary').write_bytes(b'signed candidate')
            archive=root/'notarization.zip';archive.write_bytes(b'fixed submission archive')
            sha=hashlib.sha256(archive.read_bytes()).hexdigest()
            record={'version':'0.1.1','architecture':'aarch64','team_id':'TESTTEAM01','sha256':sha,'bundle_sha256':release.bundle_digest(app)}
            def validate(r=record,expected=sha):release.validate_prepared_payload(r,expected,app,archive,'0.1.1','aarch64','TESTTEAM01')
            validate()
            for field,value in [('version','0.1.2'),('architecture','x86_64'),('team_id','OTHERTEAM1'),('sha256','0'*64),('bundle_sha256','0'*64)]:
                changed=copy.deepcopy(record);changed[field]=value
                with self.assertRaises(ValueError):validate(changed)
            for value in [None,'', '0'*64,'../archive']:
                with self.assertRaises(ValueError):validate(expected=value)
            archive.write_bytes(b'changed archive')
            with self.assertRaises(ValueError):validate()
            archive.write_bytes(b'fixed submission archive');(app/'binary').write_bytes(b'changed app')
            with self.assertRaises(ValueError):validate()
    def test_symlinks_are_not_a_reviewable_bundle(self):
        with tempfile.TemporaryDirectory() as root:
            app=Path(root);(app/'outside').symlink_to('/tmp')
            with self.assertRaises(RuntimeError):release.bundle_digest(app)
    def test_stable_manifest_has_only_the_expected_target(self):
        m=release.manifest('0.1.1','aarch64','signature','notes')
        self.assertEqual(list(m['platforms']),['darwin-aarch64'])
        self.assertEqual(m['platforms']['darwin-aarch64']['url'],'https://github.com/matsushibadenki/FlowGuard/releases/download/v0.1.1/FlowGuard_0.1.1_aarch64.app.tar.gz')
        for version in ['0.1.1-beta.1','../0.1.1','v0.1.1']:
            with self.assertRaises(ValueError):release.manifest(version,'aarch64','signature','notes')
if __name__=='__main__':unittest.main()
