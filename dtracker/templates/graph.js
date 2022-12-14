let chart;
const trackerBaseURL = "http://127.0.0.1:8080";

const getTimeRanges = (interval_in_minutes, size) => {
  const ranges = [];
  const date = new Date();

  for (let i = 0; i < size; i++) {
    const millis = i * interval_in_minutes * 60 * 1000;
    const new_date = new Date(date.getTime() - millis);
    ranges.push(new_date);
  }

  return ranges;
};

const padArrayStartWithZeroes = (arr, len) => {
  return arr.concat(Array(len - arr.length).fill(0));
};

const buildLabels = (bucket_size_in_minutes, since, groupBy) => {
  return getTimeRanges(
    bucket_size_in_minutes * groupBy,
    (since * 60) / groupBy
  );
};

const buildData = (type, contentSorted, since, groupBy) => {
  const element = type.toLowerCase();

  return padArrayStartWithZeroes(
    contentSorted.map((d) => d[element]).filter((_, i) => i % groupBy === 0),
    Math.ceil((since * 60) / groupBy)
  );
};

const buildDatasets = (content, since, groupBy) => {
  const contentSorted = content.reverse();

  return [
    {
      label: "Torrents",
      backgroundColor: "orange",
      borderColor: "orange",
      data: buildData("Torrents", contentSorted, since, groupBy),
    },
    {
      label: "Seeders",
      backgroundColor: "blue",
      borderColor: "blue",
      data: buildData("Seeders", contentSorted, since, groupBy),
    },
    {
      label: "Leechers",
      backgroundColor: "green",
      borderColor: "green",
      data: buildData("Leechers", contentSorted, since, groupBy),
    },
  ];
};

const buildChart = (content, bucket_size_in_minutes, since, groupBy) => {
  const labels = buildLabels(bucket_size_in_minutes, since, groupBy);
  const datasets = buildDatasets(
    content,
    bucket_size_in_minutes,
    since,
    groupBy
  );

  const data = {
    labels: labels,
    datasets: datasets,
  };

  const config = {
    type: "line",
    data: data,
    options: {
      elements: {
        point: {
          radius: 0,
        },
      },
      interaction: {
        mode: "index",
        intersect: false,
      },
      scales: {
        x: {
          type: "time",
          time: {
            unit: "minute",
          },
          ticks: {
            major: {
              enabled: true,
            },
          },
        },
        y: {
          beginAtZero: true,
          title: {
            display: true,
            text: "Count",
          },
        },
      },
    },
  };

  const myChart = new Chart(document.getElementById("myChart"), config);

  return myChart;
};

const fetchData = async (since) => {
  const res = await fetch(`${trackerBaseURL}/stats?since=${since}`, {
    method: "GET",
  });
  return await res.json();
};

let since = 1; // default: Ultima hora
let groupBy = 1; // default: Por minuto

const updateChart = (res_data) => {
  chart.data.labels = buildLabels(
    res_data.bucket_size_in_minutes,
    since,
    groupBy
  );

  const contentSorted = res_data.content.reverse();
  chart.legend.legendItems.forEach((item, index) => {
    if (!item.hidden) {
      chart.data.datasets[index].data = buildData(
        item.text,
        contentSorted,
        since,
        groupBy
      );
    }
  });
  chart.update("none");
};

window.addEventListener("load", async (_e) => {
  let res_data = await fetchData(since);

  chart = buildChart(
    res_data.content,
    res_data.bucket_size_in_minutes,
    since,
    groupBy
  );

  setInterval(() => {
    fetchData(since).then((res_data) => {
      updateChart(res_data);
    });
  }, 500);
});

const selectSince = document.querySelector("#since");
const selectGroupBy = document.querySelector("#groupBy");

selectSince.addEventListener("change", async (e) => {
  since = Number(e.target.value);
  const res_data = await fetchData(since);
  updateChart(res_data);
});

selectGroupBy.addEventListener("change", async (e) => {
  groupBy = Number(e.target.value);
  const res_data = await fetchData(since);
  updateChart(res_data);
});
