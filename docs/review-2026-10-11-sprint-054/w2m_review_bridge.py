from pathlib import Path
from tempfile import TemporaryDirectory
from w2m.file_bridge import FileBridge, FILE_LIST
class Mem:
 def __init__(self): self.buf=bytearray(b'\xa5'*4096)
 def read(self,a,n): return bytes(self.buf[a:a+n])
 def write(self,a,b): self.buf[a:a+len(b)]=b
with TemporaryDirectory() as d:
 root=Path(d); (root/'a').write_text('')
 f=FileBridge(root); m=Mem(); p='C:\\w2m\\app'.encode('utf-16-le'); m.write(0,p)
 need,kind=f.request(m,FILE_LIST,[0,len(p)//2,0,0])
 (root/'longer_filename').write_text('')
 new,kind=f.request(m,FILE_LIST,[0,len(p)//2,512,need])
 print('directory growth:',{'capacity':need,'second_size':new,'kind':kind,'output':m.read(512,need).hex()})
 p='C:\\w2m'.encode('utf-16-le');m.write(0,p)
 size,kind=f.request(m,FILE_LIST,[0,len(p)//2,512,1024])
 print('C:\\w2m listing:',m.read(512,size).decode('utf-16-le').split('\0'))
 print('C:\\w2m\\app resolve:',f._resolve(['w2m','app']))
 f.close_all()
