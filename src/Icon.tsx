export function Icon({kind='route',size=22}:{kind?:string;size?:number}) {
  const paths:Record<string,string> = {
    route:'M5 19a2 2 0 1 0 0-4 2 2 0 0 0 0 4Zm14-14a2 2 0 1 0 0 4 2 2 0 0 0 0-4ZM7 17h8a3 3 0 0 0 0-6H9a3 3 0 0 1 0-6h8',
    bike:'M5 20a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm14 0a4 4 0 1 0 0-8 4 4 0 0 0 0 8ZM5 16l5-9 5 9H5m9-12h3l2 12M8 7h4',
    sync:'M20 7a8 8 0 0 0-14-2L3 8m0-5v5h5M4 17a8 8 0 0 0 14 2l3-3m0 5v-5h-5',
    arrow:'M5 12h14m-6-6 6 6-6 6',
    check:'m5 12 4 4L19 6',
    account:'M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm-8 9v-2a8 8 0 0 1 16 0v2',
    cloud:'M7 18h11a4 4 0 0 0 0-8h-1a6 6 0 0 0-11-3 5 5 0 0 0 1 11Z',
  };
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={paths[kind]||paths.route}/></svg>;
}
