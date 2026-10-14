import {useEffect,useRef,useState} from 'react';
import type {AccountInfo,Progress} from './types';
import {accountAction,accountInfo} from './bridge';
import {Icon} from './Icon';
const actions={
  images:{title:'清除圖片快取',description:'移除已下載的車型圖片，保留車型文字、旅程與登入。下次更新資料會重新下載圖片。'},
  history:{title:'清除旅程快取',description:'移除本機保存的所有旅程、座標及舊快照中的旅程，保留車型與登入。雲端旅程不受影響。'},
  logout:{title:'登出帳號',description:'移除此應用程式保留的 Google 登入授權。已下載的車型與旅程仍可瀏覽；下次更新需要重新登入。'},
  all:{title:'清除所有資料並重新開始',description:'移除此應用程式的登入資訊、車型、圖片、旅程與所有舊快照，返回首次導入畫面。這項本機清除無法復原，雲端資料不受影響。'},
};
type Action=keyof typeof actions;
export function Account({progress,onChanged,onLogin}:{progress:Progress;onChanged:(error?:string)=>Promise<void>;onLogin:()=>void}){
  const [info,setInfo]=useState<AccountInfo|null>(null),[error,setError]=useState(''),[notice,setNotice]=useState(''),[pending,setPending]=useState<Action|null>(null),[working,setWorking]=useState(false);
  const dialog=useRef<HTMLDialogElement>(null);
  useEffect(()=>{let active=true;accountInfo().then(value=>{if(active)setInfo(value);}).catch(()=>{if(active)setError('無法讀取本機帳號資料。仍可登出或清除所有資料重新開始。');});return()=>{active=false;};},[progress.session,progress.phase]);
  const ask=(action:Action)=>{setPending(action);setError('');dialog.current?.showModal();};
  const cancel=()=>{dialog.current?.close();setPending(null);};
  const run=async()=>{
    if(!pending)return;
    const selected=pending;setWorking(true);setError('');
    try{await accountAction(selected);setInfo(await accountInfo());setNotice(actions[selected].title+'：已完成');cancel();await onChanged();}
    catch(e){const message=String(e instanceof Error?e.message:e);setError(message);await onChanged(message);}
    finally{setWorking(false);}
  };
  const locked=working||progress.busy;
  const stateLabel={none:'尚未登入',checking:'正在確認登入',active:'已登入',expired:'登入已失效',offline:'暫時離線'}[progress.session];
  return <section className="workspace account-workspace">
    <div className="workspace-heading"><div><span className="eyebrow">ACCOUNT & LOCAL DATA</span><h1>帳號管理</h1><p>確認登入身分，管理保存在這台電腦的資料。</p></div><span className="section-index">03 / 03</span></div>
    <section className="account-card"><div className="account-card-heading"><span className="account-avatar"><Icon kind="account" size={32}/></span><div><h2>Google 帳號</h2><p className="muted">{info?.identity?.name||'登入資訊'}</p></div><span className="badge">{stateLabel}</span></div>
      <dl className="account-details"><div><dt>Google 電子郵件</dt><dd>{info?.identity?.email||'尚無可顯示的帳號'}</dd></div><div><dt>RoadSync 帳號 ID</dt><dd className="account-id">{info?.identity?.uid||'尚未取得'}</dd></div></dl>
      <p className="hint">帳號資訊來自已驗證的 Google 登入憑證。RoadSync 帳號 ID 為旅程服務使用的使用者 ID。</p>
      {progress.session==='expired'||progress.session==='none'?<button className="btn btn-primary" disabled={locked} onClick={onLogin}>{progress.session==='expired'?'重新登入':'登入 Google 帳號'}</button>:<p className="account-status-note">{progress.session==='offline'?'目前顯示上次保存的帳號資訊。恢復連線後可繼續更新。':'登入狀態由 Windows 加密保存，重新開啟時會自動恢復。'}</p>}
      {info?.identityUnavailable&&<p className="error-text">無法解密保存的帳號，請重新登入。</p>}
    </section>
    <section className="account-card"><div className="section-heading"><h2>本機快取</h2><span className="muted">只影響這台電腦</span></div><div className="cache-counts"><div><strong>{info?.images??'—'}</strong><span>車型圖片</span></div><div><strong>{info?.days??'—'}</strong><span>行程日</span></div><div><strong>{info?.trips??'—'}</strong><span>旅程段數</span></div></div>
      <div className="account-action-row"><div><h3>車型圖片</h3><p>保留車型文字資訊與歷史旅程。</p></div><button className="btn btn-outline-secondary" disabled={locked||info?.images===0} onClick={()=>ask('images')}>清除圖片快取</button></div>
      <div className="account-action-row"><div><h3>歷史旅程</h3><p>移除本機旅程與路線，保留雲端記錄。</p></div><button className="btn btn-outline-secondary" disabled={locked||info?.days===0} onClick={()=>ask('history')}>清除旅程快取</button></div>
    </section>
    <section className="account-card account-danger"><div className="account-action-row"><div><h3>登出帳號</h3><p>清除本機登入授權，保留已下載資料。</p></div><button className="btn btn-outline-secondary" disabled={locked||(info?!info.hasSavedSession:progress.session==='none')} onClick={()=>ask('logout')}>登出帳號</button></div><div className="account-action-row"><div><h3>重新開始</h3><p>清除所有本機登入與下載資料，返回首次導入。</p></div><button className="btn btn-outline-secondary danger-button" disabled={locked} onClick={()=>ask('all')}>清除所有資料</button></div></section>
    {progress.busy&&<p className="hint" role="status">正在確認登入或下載資料，完成後即可管理本機資料。</p>}{notice&&<p className="account-notice" role="status">{notice}</p>}{error&&!pending&&<p className="error-text" role="alert">{error}</p>}
    <dialog ref={dialog} className="account-confirm" aria-labelledby="confirm-title" onCancel={event=>{if(working)event.preventDefault();else setPending(null);}}><h2 id="confirm-title">{pending&&actions[pending].title}</h2><p>{pending&&actions[pending].description}</p>{error&&<p className="error-text" role="alert">{error}</p>}<div className="confirm-actions"><button className="btn btn-outline-secondary" disabled={working} onClick={cancel} autoFocus>取消</button><button className="btn btn-primary" disabled={working} onClick={run}>{working?'正在處理…':'確認'+(pending==='logout'?'登出':'清除')}</button></div></dialog>
  </section>;
}
