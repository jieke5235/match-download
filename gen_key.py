
import pexpect
import sys

child = pexpect.spawn('npm run tauri signer generate -- -w new.key')
child.expect('Password:')
child.sendline('')
child.expect('Password:')
child.sendline('')
child.expect(pexpect.EOF)
print(child.before.decode())
