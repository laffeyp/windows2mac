from pathlib import Path
import tempfile,subprocess
from w2m.guest_runtime import api_header
root=Path('/Users/peterlaffey/port/tools/guest_runtime')
d=Path(tempfile.mkdtemp(prefix='w2m-review-windows-')); (d/'api_ids.h').write_text(api_header())
prefix='#include <stdio.h>\n#include <stdlib.h>\nstatic unsigned char test_teb[8192];\n#define W2M_TEB_VA ((unsigned long long)test_teb)\n'
body=r'''
u64 w2m_logged(u64 a,u64 b,u64 c){return c;}
void w2m_unimplemented(u64 a,u64 b){exit(88);}
u64 w2m_resource(u64 a,u32 b,u32 c,u32 *d,u32 *e){return 0;}
void *w2m_guest_frame(u64 a,u64 b,void (*c)(struct w2m_guest_call*,u64)){return calloc(1,b);}
void w2m_call_guest(struct w2m_guest_call *c,u64 f,u64 a,u64 b,u64 d,u64 e){}
void w2m_return_to(u64 a,u64 b){}
u64 user32_MonitorFromWindow(u64,u64);
u64 user32_MonitorFromRect(u64,u64);
int main(void){
 u16 name[65];for(int i=0;i<64;i++)name[i]='a';name[64]=0;
 u64 a=register_class(0,0,1,0,0,1,0,0,0,(u64)name);
 int found=find_class((u64)name);
 u64 b=register_class(0,0,1,0,0,1,0,0,0,(u64)name);
 printf("64-unit class: first=%llx lookup=%d second=%llx\n",a,found,b);
 windows[0].used=1;
 windows[0].rect[0]=2000;windows[0].rect[1]=2000;
 windows[0].rect[2]=2320;windows[0].rect[3]=2240;
 printf("offscreen window: MonitorFromWindow=%llx MonitorFromRect=%llx\n",
 user32_MonitorFromWindow(WINDOW_BASE,0),user32_MonitorFromRect((u64)windows[0].rect,0));
}
'''
(d/'main.c').write_text(prefix+f'#include "{root}/windows.c"\n'+body)
(d/'display.c').write_text(prefix+f'#include "{root}/display.c"\n')
r=subprocess.run(['clang','-O1','-I',str(d),str(d/'main.c'),str(d/'display.c'),'-o',str(d/'check')],capture_output=True,text=True)
print('build',r.returncode,r.stderr)
if not r.returncode:
 r=subprocess.run([str(d/'check')],capture_output=True,text=True);print('exit',r.returncode,r.stdout,r.stderr)
print('artifacts',d)
