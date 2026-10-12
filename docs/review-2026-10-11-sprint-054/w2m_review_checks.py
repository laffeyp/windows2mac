from pathlib import Path
import subprocess, tempfile, sys, json
sys.path.insert(0, '/Users/peterlaffey/port/src')
from w2m.guest_runtime import api_header
root=Path('/Users/peterlaffey/port')
d=Path(tempfile.mkdtemp(prefix='w2m-review-'))
print('ARTIFACTS',d,flush=True)
(d/'api_ids.h').write_text(api_header())
prefix='#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\nstatic unsigned char test_teb[8192];\n#define W2M_TEB_VA ((unsigned long long)test_teb)\n'
common='\nu64 w2m_logged(u64 a,u64 b,u64 c){return c;}\nvoid w2m_unimplemented(u64 a,u64 b){fprintf(stderr,"unimplemented %llu %llu\\n",a,b);exit(88);}\n'
registry=r'''
int main(void){
 u16 name[]={0x173,'o','f','t','w','a','r','e',0};
 u64 result=0, stack[8]={0};stack[5]=(u64)&result;
 u64 ret=advapi32_RegOpenKeyExW(HKEY_CURRENT_USER,(u64)name,0,0x20019,(u64)stack);
 printf("U+0173oftware: ret=%llu handle=%llx\n",ret,result);
 u16 nul[]={'S','o','f','t','w','a','r','e',0x100,'x',0};
 ret=advapi32_RegOpenKeyExW(HKEY_CURRENT_USER,(u64)nul,0,0x20019,(u64)stack);
 printf("Software+U+0100+x: ret=%llu handle=%llx\n",ret,result);
}
'''
search=r'''
u64 w2m_full_path(const u16 *i,u16 *o,long *p){return 0;}
u64 w2m_file_stat_full(const u16 *f,u64 o){return 2;}
u64 w2m_host(u64 op,u64 a,u64 b,u64 c,u64 d,u64 *s){return 0;}
u64 w2m_heap_take(u64 n){return (u64)malloc(n);}
void w2m_heap_release(u64 a,u64 p){free((void*)p);}
int main(void){
 struct search f={0};
 f.folder[0]='C';f.folder[1]=':';f.folder[2]='\\';
 for(int i=3;i<990;i++)f.folder[i]='a';
 u16 name[101];for(int i=0;i<100;i++)name[i]='b';name[100]=0;
 u8 data[592];fill(&f,name,(u64)data);
 puts("fill returned");
}
'''
for name,body in [('registry',registry),('file_search',search)]:
 src=d/(name+'.c')
 src.write_text(prefix+f'#include "{root}/tools/guest_runtime/{name}.c"\n'+common+body)
 exe=d/name
 r=subprocess.run(['clang','-g','-O1','-fsanitize=address','-I',str(d),str(src),'-o',str(exe)],capture_output=True,text=True)
 print(name,'build',r.returncode,r.stderr,flush=True)
 if r.returncode==0:
  r=subprocess.run([str(exe)],capture_output=True,text=True)
  (d/(name+'.out')).write_text(r.stdout+r.stderr)
  print(name,'exit',r.returncode,r.stdout,r.stderr[:4500],flush=True)
