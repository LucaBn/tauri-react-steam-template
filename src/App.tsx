import { useState } from "react";

function App() {
  const [counter, setCounter] = useState<number>(0);
  return (
    <>
      <p>App name</p>
      <p>Value: {counter}</p>
      <button onClick={() => setCounter((prev) => prev + 1)}>
        Increase by 1
      </button>
    </>
  );
}

export default App;
