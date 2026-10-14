import {useEffect,useState} from 'react';
import type {Progress,Snapshot} from './types';
import {loadSnapshot,startSync,status,serviceConfigured} from './bridge';
import {ServiceSetup} from './ServiceSetup';
import {Onboarding} from './Onboarding';
import {Catalog} from './Catalog';
import {Journeys} from './Journeys';
import {Icon} from './Icon';
import {Account} from './Account';
const idle:Progress={phase:'idle',message:'',busy:false,error:null,loginUrl:null,session:'none',catalog:{completed:0,total:null,message:''},history:{completed:0,total:null,message:''}};
function errorText(code:string) {
  const known:Record<string,string>={
    SERVICE_CONFIG_MISSING:'請匯入 RoadSync 服務設定後再登入。既有資料仍可查看。',
    SERVICE_CONFIG_INVALID:'服務設定格式不正確，請重新匯入。既有資料仍可查看。',
    DESKTOP_APP_REQUIRED:'請從 OpenRideSync Viewer 桌面應用程式啟動登入。',
    LOGIN_TIMEOUT:'登入已逾時，請重新開啟登入。',
    BROWSER_OPEN_FAILED:'無法開啟系統瀏覽器。請確認預設瀏覽器後重試。',
    AUTH_INVALID:'登入已過期或無法驗證，請重新登入。既有資料仍可查看。',
    SESSION_EXPIRED:'登入已失效，請重新登入。既有車型與旅程仍可查看。',
    SESSION_UNREADABLE:'無法恢復這台電腦的登入，請重新登入。既有資料仍可查看。',
    SESSION_OFFLINE:'暫時無法連線，登入狀態已保留。可以繼續查看本機資料，稍後再更新。',
    AUTH_KEYS_UNAVAILABLE:'暫時無法驗證登入，請稍後重試。既有資料仍可查看。',
    SESSION_STORAGE_FAILED:'無法保存登入狀態，請確認本機儲存空間與權限後重試。',
    CLOUD_RPC_Unauthenticated:'雲端登入已失效，請重新登入。既有資料仍可查看。',
    HISTORY_ACCOUNT_MISMATCH:'旅程帳號與登入身分不符，本次資料未保存。',
  };
  return known[code]||'本次讀取未完成，請確認網路後重試。錯誤代碼：'+code;
}
export default function App() {
  const [snapshot,setSnapshot]=useState<Snapshot|null>(null),[progress,setProgress]=useState(idle);
  const [initializing,setInitializing]=useState(true),[onboarding,setOnboarding]=useState(true),[error,setError]=useState('');
  const [view,setView]=useState<'catalog'|'history'|'account'>('history'),[modelFocus,setModelFocus]=useState('');
  const [configured,setConfigured]=useState(true),[settingUp,setSettingUp]=useState(false);
  useEffect(()=>{
    let active=true;
    Promise.allSettled([loadSnapshot(),status(),serviceConfigured()]).then(([cached,current,service])=>{
      if(!active)return;
      if(cached.status==='fulfilled'){setSnapshot(cached.value);setOnboarding(!cached.value||(import.meta.env.DEV&&new URLSearchParams(location.search).has('onboarding')));}
      else setError(errorText(String(cached.reason)));
      if(current.status==='fulfilled'){setProgress(current.value);if(current.value.error)setError(errorText(current.value.error));}
      setConfigured(service.status==='fulfilled'&&service.value);
      setInitializing(false);
    });
    return()=>{active=false;};
  },[]);
  useEffect(()=>{
    if(!progress.busy)return;
    let active=true;
    const timer=setInterval(async()=>{
      try{
        const p=await status(),fresh=p.phase==='done'?await loadSnapshot():null;
        if(!active)return;
        if(fresh)setSnapshot(fresh);
        if(p.error)setError(errorText(p.error));
        setProgress(p);
      }catch{
        if(active){setProgress(p=>({...p,busy:false}));setError('無法讀取應用程式狀態，請重新開啟。');}
      }
    },700);
    return()=>{active=false;clearInterval(timer);};
  },[progress.busy]);
  const begin=async()=>{
    if(!configured){setSettingUp(true);return;}
    setError('');if(!snapshot)setOnboarding(true);
    try{await startSync(progress.session==='expired');setProgress(await status());}
    catch(e){setError(errorText(e instanceof Error?e.message:String(e)));}
  };
  const navigate=(target:'catalog'|'history'|'account',label='')=>{setView(target);setModelFocus(label);};
  const localChanged=async(message?:string)=>{
    const fresh=await loadSnapshot();setSnapshot(fresh);setProgress(await status());setConfigured(await serviceConfigured());setModelFocus('');
    if(message)setError('本機操作未完整完成：'+message);
    if(!fresh)setOnboarding(true);
  };
  const refreshLabel=progress.busy?'正在讀取…':progress.session==='expired'?'重新登入':'更新資料';
  const sessionLabel={none:'更新時連結 Google 帳號',checking:'正在恢復登入狀態',active:'登入狀態已保存',expired:'登入已失效 · 本機可用',offline:'連線暫不可用 · 本機可用'}[progress.session];
  return <div className="app-shell">
    <header className="app-header"><div className="brand"><span className="brand-symbol"><Icon size={25}/></span><div>OPENRIDESYNC <strong>VIEWER</strong><span>THE DESKTOP RIDING COLLECTION</span></div></div><span className="header-caption">LOCAL COLLECTION <i/> READ ONLY</span>{onboarding&&!initializing&&<button className="btn btn-outline-secondary" onClick={()=>{setSettingUp(false);setOnboarding(false);navigate('account');}}>帳號管理</button>}{!onboarding&&<button className="btn btn-outline-secondary compact-refresh" disabled={progress.busy} onClick={begin} aria-label={refreshLabel}><Icon kind="sync" size={18}/></button>}</header>
    {initializing?<div className="empty-state" role="status">正在開啟本機資料…</div>:(settingUp||(!configured&&onboarding))?<ServiceSetup onCancel={snapshot?()=>{setSettingUp(false);setOnboarding(false);}:undefined} onConfigured={()=>{setConfigured(true);setSettingUp(false);setError('');}}/>:onboarding?<Onboarding progress={progress} error={error} ready={progress.phase==='done'?snapshot:null} onStart={begin} onEnter={()=>{navigate('history');setOnboarding(false);}}/>:<div className="workspace-shell">
      <aside className="viewer-sidebar"><div className="viewer-sidebar-title">探索資料</div><nav aria-label="主要導覽"><button className={view==='catalog'?'active':''} aria-current={view==='catalog'?'page':undefined} disabled={!snapshot} onClick={()=>navigate('catalog')}><Icon kind="bike"/><span>車型圖鑑</span><small>{snapshot?.models.length||0}</small></button><button className={view==='history'?'active':''} aria-current={view==='history'?'page':undefined} disabled={!snapshot} onClick={()=>navigate('history')}><Icon/><span>歷史旅程</span><small>{snapshot?.summary.days||0}</small></button><button className={view==='account'?'active':''} aria-current={view==='account'?'page':undefined} onClick={()=>navigate('account')}><Icon kind="account"/><span>帳號管理</span></button></nav>
        <div className="viewer-sidebar-bottom"><span className="connection-dot"/> {snapshot?'本機快照已就緒':'尚未下載資料'}<p>最後讀取<br/>{snapshot?new Date(snapshot.updatedAt).toLocaleString('zh-TW',{timeZone:'Asia/Taipei',hour12:false}):'—'}</p><button className="btn btn-outline-secondary refresh-button" onClick={begin} disabled={progress.busy}><Icon kind="sync" size={17}/>{refreshLabel}</button><p>{sessionLabel}<br/>只讀取既有雲端資料。</p><small>VERSION 0.1.0</small></div>
      </aside>
      <main>{view==='account'?<Account progress={progress} onChanged={localChanged} onLogin={begin}/>:snapshot&&(view==='catalog'?<Catalog snapshot={snapshot} focus={modelFocus} onClearFocus={()=>setModelFocus('')} onHistory={label=>navigate('history',label)}/>:<Journeys snapshot={snapshot} focus={modelFocus} onClearFocus={()=>setModelFocus('')} onCatalog={label=>navigate('catalog',label)}/>)}</main>
      {(progress.busy||error||progress.phase==='done')&&<div className={'status-toast '+(error?'error':'')} role="status"><strong>{error||progress.message}</strong>{progress.session==='expired'&&!progress.busy&&<button className="btn btn-outline-secondary" onClick={begin}>重新登入</button>}{!progress.busy&&<button aria-label="關閉通知" onClick={()=>{setError('');setProgress(p=>({...p,phase:'idle',message:'',error:null}));}}>×</button>}</div>}
    </div>}
    <footer className="app-footer"><span>OPENRIDESYNC VIEWER</span><span>非 Honda 官方應用程式 · 只讀取既有資料</span></footer>
  </div>;
}
