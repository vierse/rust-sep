import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  BarElement,
  Title,
  Tooltip,
  Legend,
} from 'chart.js';
import { Bar } from 'react-chartjs-2';
import { TextField, Box, IconButton, Button } from "@radix-ui/themes";
import React from "react";

import { getReq } from "/src/api";
import { CategorySelecter, allOptions } from "./CategorySelecter";

ChartJS.register(
  CategoryScale,
  LinearScale,
  BarElement,
  Title,
  Tooltip,
  Legend
);


export function MainView() {

  const [useData, setData] = React.useState(Array(7).fill(0));
  const [catState, setCatState] = React.useState([null]);

  const get_data = async (cats) => {
    console.debug("cats=" + cats);
    const res = await getReq("/metrics/data",
                          null,
                          new URLSearchParams({weekdays: cats.join(',')}));
    setData(res);
  };

  const onSelectValues = (value, index) => {
    const clonedCatState = structuredClone(catState);

    clonedCatState[index] = value;
    setCatState(clonedCatState);
    get_data(clonedCatState[0].map((opt) => opt.value));
  };

 
  const weekdays = ["Monday", "Tuesday", "Wendsday", "Thursday", "Friday", "Saturday", "Sunday"];
  const data = {
    labels: weekdays,
    datasets: [
      {
        label: 'totals',
        data: useData,
        backgroundColor: "red",
      },
    ],
  };

  return (
    <>
      {catState.map((selectCount, index) => {
        const options = getOptionsToRender(catState, allOptions);
        return (
            <CategorySelecter
              value={catState[index]}
              options={options}
              onSelect={(value) => onSelectValues(value, index)}
              key={index}
            />
        );
      })}
      <Bar data={data} />
    </>
  )
}

const getOptionsToRender = (allSelectedOptions, allOptions) => {
  const filteredOptions = allSelectedOptions.flatMap((options) => options);

  const optionsToRender =
      filteredOptions.length > 0 ? allOptions.filter(
            (option) => !filteredOptions.some((selectOption) =>
                option && selectOption && option.value == selectOption.value
              )
            ): allOptions;
  return optionsToRender;
}
