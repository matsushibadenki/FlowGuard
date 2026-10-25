import unittest,base64,copy
from merge_update_feeds import merge_manifests
class MergeTests(unittest.TestCase):
 def feeds(self):
  version='1.2.0';return [{'version':version,'notes':'same','pub_date':'2026-01-01T00:00:00Z','platforms':{'darwin-'+arch:{'signature':base64.b64encode(b'trusted comment: version:1.2.0').decode(),'url':f'https://github.com/matsushibadenki/FlowGuard/releases/download/v{version}/FlowGuard_{version}_{arch}.app.tar.gz'}}} for arch in ['aarch64','x86_64']]
 def test_merge_requires_both_targets_and_exact_version_notes_url_signature(self):
  feeds=self.feeds();self.assertEqual(set(merge_manifests(feeds)['platforms']),{'darwin-aarch64','darwin-x86_64'})
  for changed in [feeds[:1],[feeds[0],feeds[0]]]:
   with self.assertRaises(ValueError):merge_manifests(changed)
  for field,value in [('version','1.2.1'),('notes','other')]:
   changed=copy.deepcopy(feeds);changed[1][field]=value
   with self.assertRaises(ValueError):merge_manifests(changed)
  changed=copy.deepcopy(feeds);changed[1]['platforms']['darwin-x86_64']['url']='https://evil.invalid/app.app.tar.gz'
  with self.assertRaises(ValueError):merge_manifests(changed)
if __name__=='__main__':unittest.main()
