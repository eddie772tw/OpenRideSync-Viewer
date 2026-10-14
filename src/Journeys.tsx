import {useEffect,useMemo,useRef,useState} from 'react';
import type {Day,Snapshot} from './types';
import {filterDays} from './filter.mjs';
import {km,duration,speed,time,modelName,modelImage,generation} from './format';
import {RouteMap} from './RouteMap';
import {Icon} from './Icon';
export function Journeys({snapshot,focus,onClearFocus,onCatalog}:{snapshot:Snapshot;focus:string;onClearFocus:()=>void;onCatalog:(label:string)=>void}) {
  const [query,setQuery]=useState(''),[model,setModel]=useState(''),[from,setFrom]=useState(''),[to,setTo]=useState('');
  const [date,setDate]=useState(''),[trip,setTrip]=useState(-1);
  const days=useMemo(()=>filterDays(snapshot.days,{query,model:focus||model,from,to}) as Day[],[snapshot,query,model,focus,from,to]);
  const day=days.find(d=>d.date===date)||days[0];
  const detail=useRef<HTMLElement>(null);
  useEffect(()=>{detail.current?.scrollTo({top:0});},[day?.date]);
  const index=useMemo(()=>new Map(snapshot.models.map(m=>[m.modelLabel,m])),[snapshot]);
  const available=new Set(snapshot.days.flatMap(d=>d.vehicle.models.map(m=>m.label)));
  const reset=()=>{setQuery('');setModel('');setFrom('');setTo('');onClearFocus();setTrip(-1);};
  const change=()=>{setTrip(-1);};
  return <section className="workspace journey-workspace">
    <div className="workspace-heading"><div><span className="eyebrow">YOUR RIDING JOURNAL</span><h1>歷史旅程<span className="count-label">{snapshot.summary.trips} RIDES</span></h1><p>回看走過的路，也看見每一段旅程背後的車款。</p></div><div className="section-index">02 / 03</div></div>
    <div className="journey-stats">{[[days.length,'行程日'],[days.reduce((n,d)=>n+d.trips.length,0),'旅程段數'],[km(days.reduce((n,d)=>n+d.distance,0)),'公里'],[duration(days.reduce((n,d)=>n+d.duration,0)),'記錄時間']].map(([value,label])=><div className="stat" key={label}><span>{label}</span><strong>{value}</strong></div>)}</div>
    <div className="filters journey-filters">
      <label>車名或代碼<input className="form-control" type="search" value={query} placeholder="CBR500R、MLRA" onChange={e=>{setQuery(e.target.value);change();}}/></label>
      <label>對照車型<select className="form-select" value={focus||model} onChange={e=>{onClearFocus();setModel(e.target.value);change();}}><option value="">全部車型</option><option value="unmatched">尚未對應</option>{[...available].map(label=>{const m=index.get(label);return m&&<option key={label} value={label}>{modelName(m)+' · '+m.modelYears.join('/')}</option>;})}</select></label>
      <label>開始日期<input className="form-control" type="date" value={from} onChange={e=>{setFrom(e.target.value);change();}}/></label>
      <label>結束日期<input className="form-control" type="date" value={to} onChange={e=>{setTo(e.target.value);change();}}/></label>
      <button className="btn btn-outline-secondary" onClick={reset}>清除篩選</button>
    </div>
    <div className="result-line"><span role="status">{from&&to&&from>to?'開始日期不能晚於結束日期':'顯示 '+days.length+' / '+snapshot.days.length+' 個行程日'}</span><span>台北時間 · km / km/h</span></div>
    {day?<div className="journey-layout">
      <nav className="day-list" aria-label="行程日期">{days.map(d=><button className={'day-button '+(d===day?'selected':'')} key={d.date} aria-pressed={d===day} onClick={()=>{setDate(d.date);setTrip(-1);}}>
        <span className="day-date">{d.date}<Icon kind="arrow" size={16}/></span><strong>{km(d.distance)} <small>km</small></strong><span className="day-sub">{d.vehicle.names.join('、')||'未提供車名'}<span>{d.trips.length} 段</span></span>
      </button>)}</nav>
      <article className="journey-detail" ref={detail} tabIndex={0} aria-label="旅程詳細資料">
        <div className="detail-heading"><div><span className="eyebrow">RIDE DETAILS</span><h2>{day.date}</h2><p>{day.vehicle.names.join('、')||'未提供車名'}<span className="separator">/</span>{time(day.start)}–{time(day.end)}</p></div><span className="badge">已保存</span></div>
        <div className="ride-metrics"><div><strong>{km(day.distance)}</strong><span>km · 距離</span></div><div><strong>{speed(day.avgSpeed)}</strong><span>km/h · 均速</span></div><div><strong>{duration(day.duration)}</strong><span>記錄時間</span></div></div>
        <RouteMap trips={day.trips} selected={trip<day.trips.length?trip:-1}/>
        <section className="detail-section"><div className="section-heading"><h3>旅程分段 <span>{day.trips.length}</span></h3><button className="text-button" onClick={()=>setTrip(-1)} aria-pressed={trip===-1}>顯示整日路線</button></div>
          <div className="trip-list">{day.trips.map((t,i)=><button className={'trip-button '+(trip===i?'selected':'')} aria-pressed={trip===i} key={i} onClick={()=>setTrip(i)}><span className="trip-index">{String(i+1).padStart(2,'0')}</span><span><strong>{time(t.start)}–{time(t.end)}</strong><small>{t.points.length} 點{t.coarsePoints?' · '+t.coarsePoints+' 個粗略點位':''}</small></span><span className="trip-values"><strong>{km(t.distance)} km</strong><small>{speed(t.avgSpeed)} km/h</small></span></button>)}</div>
          <p className="hint">路線依保存點位繪製；分段時間為首末點位範圍，未補造道路軌跡。</p>
        </section>
        <section className="detail-section"><div className="section-heading"><h3>車型對照</h3><span className="muted">{day.vehicle.codes.join(' / ')}</span></div>
          <p className="match-note">{day.vehicle.nameConflict?'代碼與車名不一致，以下僅為代碼候選。':day.vehicle.basis==='code+name'?'車名與代碼相符；圖鑑涵蓋的年式不代表已確認實際年式。':day.vehicle.basis==='name'?'僅車名相符，以下為候選車型；尚無法確認型號與年式。':day.vehicle.basis==='code'?'以下為代碼吻合的候選；共用代碼可能對應多個車款。':'此行程缺少足夠的車名或代碼，尚未對應車型。'}</p>
          {day.vehicle.models.map(c=>{const m=index.get(c.label);return m&&<div className="match-card" key={c.label}><div className="match-image">{modelImage(m,snapshot.images)&&<img src={modelImage(m,snapshot.images)} alt={modelName(m)}/>}</div><div><span className="eyebrow">{m.modelLabel} · {generation(m)}</span><h4>{modelName(m)}</h4><p>圖鑑年式 {m.modelYears.join(' / ')} · 儀表 type {m.meter.type}</p><button className="text-button" onClick={()=>onCatalog(m.modelLabel)}>查看車型資料 <Icon kind="arrow" size={16}/></button></div></div>;})}
        </section>
      </article>
    </div>:<div className="empty-state"><Icon size={40}/><h2>{snapshot.days.length?'沒有符合條件的旅程' :snapshot.cacheState?.historyCleared?'旅程快取已清除':'尚無歷史旅程'}</h2><p>{snapshot.days.length?'調整日期或車型篩選，再看看其他記錄。':snapshot.cacheState?.historyCleared?'按「更新資料」即可重新下載雲端旅程。':'目前帳號沒有既有旅程，仍可瀏覽完整車型圖鑑。'}</p><button className="btn btn-outline-secondary" onClick={reset}>清除篩選</button></div>}
    <p className="workspace-footnote">OpenStreetMap 提供街道底圖；圖磚請求包含檢視區域與 IP。旅程點位清單不會傳送至地圖服務。</p>
  </section>;
}
