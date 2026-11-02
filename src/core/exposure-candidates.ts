// Port-number hints only; they do not identify the service or its exposure.
const candidates: Readonly<Record<number,string>> = {
  21:'FTP',22:'SSH',23:'Telnet',25:'SMTP',80:'HTTP',110:'POP3',139:'NetBIOS',143:'IMAP',
  443:'HTTPS',445:'SMB',587:'SMTP',1433:'SQL Server',2375:'Docker API',2376:'Docker API',
  3306:'MySQL',3389:'RDP',5432:'PostgreSQL',6379:'Redis',8000:'HTTP',8080:'HTTP',
  8443:'HTTPS',11211:'Memcached',27017:'MongoDB',
};
export function exposureCandidate(port:number):string|null{return candidates[port]??null;}
