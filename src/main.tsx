// パス: src/main.tsx
import {createRoot} from 'react-dom/client';
import {App} from './App';
import {initialize} from './core/store';
import './styles.css';
import './styles/map.css';
void initialize();
createRoot(document.getElementById('root')!).render(<App/>);
