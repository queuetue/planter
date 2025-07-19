import './App.css'
import { BrowserRouter as Router, Routes, Route, Link, Outlet, useParams } from 'react-router-dom';

function TabsLayout() {
  return (
    <div>
      <h2>Tabs Layout</h2>
      <Outlet />
    </div>
  );
}

function HomeTabContent() {
  return (
    <div>
      <h3>Home Tab</h3>
      <p>This is the content of the Home tab.</p>
    </div>
  );
}

function SettingsTabContent() {
  return (
    <div>
      <h3>Settings Tab</h3>
      <p>This is the content of the Settings tab.</p>
    </div>
  );
}


function App() {
  return (
    <Router>
      <nav>
        <Link to="home">Home</Link> - <Link to="settings">Settings</Link>
      </nav>
      <Routes>
        <Route path="/" element={<TabsLayout />}>
          <Route path="home" element={<HomeTabContent />} />
          <Route path="settings" element={<SettingsTabContent />} />
        </Route>
        {/* Other routes */}
      </Routes>
    </Router>
  );
}

export default App
