import type {DownloadProgress,Progress,Snapshot} from './types';
import {Icon} from './Icon';
function DownloadRing({title,progress}:{title:string;progress:DownloadProgress}) {
  const percent=progress.total===null?null:progress.total===0?100:Math.min(100,Math.floor(progress.completed/progress.total*100));
  return <div className="download-item">
    <div className={'download-ring '+(percent===null?'indeterminate':'')} role="progressbar" aria-label={title} aria-valuemin={0} aria-valuemax={100} aria-valuenow={percent??undefined} aria-valuetext={progress.message||'準備下載'}>
      <svg viewBox="0 0 180 180" aria-hidden="true"><circle className="ring-track" cx="90" cy="90" r="76"/><circle className="ring-value" cx="90" cy="90" r="76" pathLength="100" strokeDasharray="100" strokeDashoffset={percent===null?75:100-percent}/></svg>
      <div className="ring-number">{percent===null?<span className="ring-reading">讀取中</span>:<><strong>{percent}</strong><span>%</span></>}</div>
    </div><h3>{title}</h3><p>{progress.message||'準備下載'}</p>
  </div>;
}
export function Onboarding({progress,error,ready,onStart,onEnter}:{progress:Progress;error:string;ready:Snapshot|null;onStart:()=>void;onEnter:()=>void}) {
  const downloading=progress.phase==='download'||progress.phase==='saving'||(!ready&&(progress.catalog.total!==null||progress.history.total!==null));
  const step=ready?3:downloading?2:1;
  return <div className={'onboarding '+(ready?'is-ready':downloading?'is-downloading':'')}>
    <div className="onboarding-copy">
      <span className="eyebrow">{ready?'READY TO EXPLORE':downloading?'DOWNLOAD IN PROGRESS':'你的車款。你的每一段旅程。'}</span>
      <h1>{ready?<>資料已就緒，<br/><span>回看每一段旅程。</span></>:downloading?<>正在下載，<br/><span>你的騎乘記錄。</span></>:<>每一段騎乘，<br/><span>都值得回看。</span></>}</h1>
      <p className="lead">{ready?<>車型與旅程已安全保存在本機。<br/>你的騎乘收藏，現在就能開始探索。</>:downloading?<>車型與歷史旅程正在同時下載。<br/>兩項完成後，就能開始探索。</>:<>把 RoadSync 裡的車型與旅程帶到桌面。<br/>沿著地圖，重新探索你的騎乘記錄。</>}</p>
      {ready?<div className="ready-art" role="img" aria-label="資料準備就緒"><svg viewBox="0 0 220 220" fill="none" aria-hidden="true"><circle cx="110" cy="110" r="94"/><path d="m62 111 31 31 64-68"/></svg><strong>準備就緒</strong></div>:
        downloading?<div className="download-rings"><DownloadRing title="車型與圖片" progress={progress.catalog}/><DownloadRing title="歷史旅程" progress={progress.history}/></div>:
        <div className="onboard-art" aria-hidden="true">
          <svg viewBox="0 0 640 230" fill="none"><path className="art-grid" d="M0 45h640M0 90h640M0 135h640M0 180h640M80 0v230M160 0v230M240 0v230M320 0v230M400 0v230M480 0v230M560 0v230"/>
            <path className="art-route" d="M35 175h85l55-65h100l55 45h75l45-85h95l50-35"/>
            <circle className="art-start" cx="35" cy="175" r="7"/><circle className="art-end" cx="595" cy="35" r="7"/>
          </svg><span>CONNECT · EXPLORE · REVISIT</span>
        </div>}
      <div className="feature-line"><Icon kind="bike"/><span>完整車型圖鑑</span><Icon/><span>歷史路線地圖</span><Icon kind="cloud"/><span>本機保存</span></div>
    </div>
    <section className="onboarding-panel" aria-labelledby="setup-title">
      <span className="eyebrow">STEP 0{step} / 03</span><h2 id="setup-title">{ready?'一切準備就緒':downloading?'正在下載你的資料':'準備你的騎乘資料'}</h2>
      <ol className="steps">
        {[['登入 RoadSync','使用原本的 Google 帳號'],['下載資料','車型與旅程同時下載'],['開始探索','在桌面檢視你的騎乘記錄']].map(([title,description],i)=><li key={title} className={step===i+1?'active':step>i+1?'complete':''} aria-current={step===i+1?'step':undefined}><span className="step-number">{step>i+1?<Icon kind="check" size={17}/>:String(i+1).padStart(2,'0')}</span><div><strong>{title}</strong><p>{description}</p></div></li>)}
      </ol>
      <div className="onboarding-status" role="status" aria-live="polite">
        {ready?<><strong>下載完成 · 已保存在本機</strong><p>{ready.models.length} 個型號 · {ready.summary.days} 個行程日 · {ready.summary.trips} 段旅程</p></>:progress.busy?<><strong className="loading-text">{progress.message}</strong><p>{progress.phase==='login'?'瀏覽器已開啟，完成登入後會自動繼續。':downloading?'左側顯示各項實際進度，請保持應用程式開啟。':'正在確認你的登入，稍後會自動開始下載。'}</p></>:<p>首次使用需連上網路。完成下載後，旅程與車型資料可直接從本機開啟。</p>}
        {error&&<p className="error-text" role="alert">{error}</p>}
      </div>
      <button className="btn btn-primary primary-action" disabled={progress.busy} onClick={ready?onEnter:onStart}>{ready?'開啟我的旅程':progress.busy?(downloading?'正在下載資料…':'正在確認登入…'):progress.session==='active'?'繼續下載資料':progress.session==='expired'?'重新登入 Google 帳號':'以 Google 帳號登入'}<Icon kind="arrow" size={20}/></button>
      <p className="privacy-note">登入在系統瀏覽器完成，登入狀態由 Windows 加密保存在本機。授權失效時再重新登入，仍可查看已下載的資料。地圖預設載入 OpenStreetMap 街道底圖。</p>
    </section>
  </div>;
}
