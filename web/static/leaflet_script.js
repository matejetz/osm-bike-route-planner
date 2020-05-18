// TODO: This needs major refactoring, create objects for edges, remove global variables, pull out methods
var map = L.map('map', {
    maxBounds: [
        [47.3, 5.9], // Southwest coordinates
        [54.9, 16.9512215] // Northeast coordinates
    ],
}).setView([51.1657, 10.4515], 6);
L.tileLayer('https://api.tiles.mapbox.com/v4/{id}/{z}/{x}/{y}.png?access_token={accessToken}', {
    attribution: 'Map data &copy; <a href="https://www.openstreetmap.org/">OpenStreetMap</a> contributors, <a href="https://creativecommons.org/licenses/by-sa/2.0/">CC-BY-SA</a>, Imagery © <a href="https://www.mapbox.com/">Mapbox</a>',
    maxZoom: 18,
    minZoom: 6,
    id: 'mapbox.streets',
    accessToken: 'pk.eyJ1IjoibWFwYm94IiwiYSI6ImNpejY4NXVycTA2emYycXBndHRqcmZ3N3gifQ.rJcFIG214AriISLbB6B5aw'
}).addTo(map);

map.on('click', onMapClick);

let url = "http://localhost:8080/";

let startPoint;
let startMarker;
let endPoint;
let endMarker;
let tmpMarker;
let pathsOnMap = [];
let result;
let lineChart;
let scatterChart;
let xhr = new XMLHttpRequest();

function onMapClick(e) {
    if (tmpMarker) {
        map.removeLayer(tmpMarker);
    }
    tmpMarker = L.marker(e.latlng).addTo(map);
    tmpMarker.setLatLng(e.latlng);
    tmpMarker.bindPopup("<button class='set-point set-start' onclick='setStart()''>Set Start</button><button class='set-point set-end' onclick='setEnd()''>Set End</button>").openPopup();
}

function setStart() {
    let coords = tmpMarker.getLatLng();
    let lat = Math.round(coords.lat * 1000) / 1000;
    let lng = Math.round(coords.lng * 1000) / 1000;
    document.getElementById("start-text").innerHTML = "latitude: " + lat.toString() + "<br> longitude: " + lng.toString();
    if (startMarker) {
        map.removeLayer(startMarker);
    }
    startPoint = tmpMarker.getLatLng();
    startMarker = L.marker(coords, {
        icon: greenIcon
    }).addTo(map);
    map.removeLayer(tmpMarker);
    if (pathsOnMap.length > 0) {
        for (let path of pathsOnMap) {
            map.removeLayer(path);
        }
        pathsOnMap = []
    }
}

function setEnd() {
    let coords = tmpMarker.getLatLng();
    let lat = Math.round(coords.lat * 1000) / 1000;
    let lng = Math.round(coords.lng * 1000) / 1000;
    document.getElementById("end-text").innerHTML = "latitude: " + lat.toString() + "<br> longitude: " + lng.toString();
    if (endMarker) {
        map.removeLayer(endMarker);
    }
    endPoint = tmpMarker.getLatLng();
    endMarker = L.marker(coords, {
        icon: redIcon
    }).addTo(map);
    map.removeLayer(tmpMarker);
    if (pathsOnMap.length > 0) {
        for (let path of pathsOnMap) {
            map.removeLayer(path);
        }
        pathsOnMap = []
    }
}

function showElevationGraph(elevationPoints, totalElevation, color) {
    console.log(elevationPoints);
    let result = document.getElementById('totalElevation');
    result.innerText = 'Total: ' + totalElevation.toFixed(2) + 'm';
    let graph = document.getElementById('elevationGraph').getContext('2d');
    lineChart = new Chart(graph, {
        type: 'line',
        data: {
            labels: elevationPoints.map((ele, index) => index),
            datasets: [{
                label: 'Elevation in m',
                data: elevationPoints,
                backgroundColor: color,
                borderColor: color,
                fill: false,
                pointRadius: 0
            }]
        },
        options: {
            scales: {
                yAxes: [{
                    ticks: {
                        beginAtZero: false
                    }
                }]
            }
        }
    });
    let graphContainer = document.getElementById("elevationGraphContainer");
    graphContainer.style.display = "block";
}

function displayResponse(responseText) {
    let json = JSON.parse(responseText);
    // result is ordered by ascending length
    if (json.length === 0) {
        showNoPathFound();
        return;
    }
    // shortest distance result always first
    for (jsonResult of json) {
        result.add(jsonResult);
    }
    showResultToast(result);
    createViews(result);
}


function removeGraphsAndEdges() {
    if (lineChart) {
        lineChart.destroy();
    }
    if (scatterChart) {
        scatterChart.destroy();
    }
    if (pathsOnMap.length > 0) {
        for (let path of pathsOnMap) {
            map.removeLayer(path);
        }
    }
    pathsOnMap = []
}

function query() {
    hideResult();
    hideInvalidRequest();
    hideNoPathFound();
    hideSelectStartAndEnd();
    removeGraphsAndEdges();
    result = new Result();

    if (pathsOnMap.length > 1) {
        for (let path of pathsOnMap) {
            map.removeLayer(path);
        }
        pathsOnMap = []
    }

    if (typeof startPoint === 'undefined' || typeof endPoint === 'undefined') {
        showSelectStartAndEnd();
        return;
    }

    let xhr = new XMLHttpRequest();
    xhr.open("POST", url + "dijkstra", true);
    xhr.setRequestHeader("Content-type", "application/json;charset=UTF-8");

    xhr.onreadystatechange = function () {
        console.log('complete response', xhr.responseText);

        if (xhr.readyState === 4 && xhr.status === 200) {
            displayResponse(xhr.responseText)
        } else if (xhr.readyState === 4) {
            showInvalidRequest();
        }
    };

    let travelType = document.getElementById("travel-type").value;
    let optimization = document.getElementById("optimization").value === "distance";
    let maxElevation = parseInt(document.getElementById("max-elevation").value);
    let allPaths = document.getElementById("all-paths").checked
    let body = {
        "start": {
            "latitude": startPoint.lat,
            "longitude": startPoint.lng
        },
        "end": {
            "latitude": endPoint.lat,
            "longitude": endPoint.lng
        },
        "travel_type": travelType,
        "by_distance": optimization,
        "max_ele_rise": maxElevation,
        "all_paths": allPaths
    };
    let data = JSON.stringify(body);
    // console.log("request: " + data);
    xhr.send(data);
}

function createScatterChart(result) {
    let graph = document.getElementById('scatterGraph').getContext('2d');
    datasetArray = [];
    for (resultPath of result.weightedResults) {
        datasetArray.push({
            data: [{
                x: resultPath.distance,
                y: resultPath.elevation,
            }],
            backgroundColor: resultPath.color,
            borderColor: resultPath.color,
        })
    }
    scatterChartChart = new Chart(graph, {
        type: 'scatter',
        data: {
            datasets: datasetArray
        },
        options: {
            legend: {display: false},
            scales: {
                yAxes: [{
                    scaleLabel: {
                        display: true,
                        labelString: 'elevation in m',
                    },
                    ticks: {
                        beginAtZero: false
                    }
                }],
                xAxes: [{
                    scaleLabel: {
                        display: true,

                        labelString: `distance in ${result.distanceType}`,
                    },

                }]
            }
        }
    });
    let graphContainer = document.getElementById("scatterGraphContainer");
    graphContainer.style.display = "block";
}

function createViews(result) {
    createScatterChart(result);
    for (let resultPath of result.weightedResults) {
        createPathView(resultPath.path, resultPath.distance, resultPath.distance_type, resultPath.elevation, resultPath.color);
    }
}


function showInvalidRequest() {
    document.getElementById("invalid-request").style.display = "block";
}

function hideInvalidRequest() {
    var x = document.getElementById("invalid-request");
    if (x.style.display === "block") {
        x.style.display = "none";
    }
}

function showNoPathFound() {
    document.getElementById("no-path-found").style.display = "block";
}

function hideNoPathFound() {
    var x = document.getElementById("no-path-found");
    if (x.style.display === "block") {
        x.style.display = "none";
    }
}

function showSelectStartAndEnd() {
    document.getElementById("select-start-and-end").style.display = "block";
}

function hideSelectStartAndEnd() {
    var x = document.getElementById("select-start-and-end");
    if (x.style.display === "block") {
        x.style.display = "none";
    }
}

function showResultToast(result) {
    var tmp = document.getElementById("result");
    tmp.innerHTML = `Shortest path for elevation restriction has ${result.minDistance} ${result.distanceType}`;
    tmp.style.display = "block";
}

function hideResult() {
    var x = document.getElementById("result");
    if (x.style.display === "block") {
        x.style.display = "none";
    }
}

var greenIcon = new L.Icon({
    iconUrl: 'img/marker-green.png',
    shadowUrl: 'img/marker-shadow.png',
    iconSize: [25, 41],
    iconAnchor: [12, 41],
    popupAnchor: [1, -34],
    shadowSize: [41, 41]
});
var redIcon = new L.Icon({
    iconUrl: 'img/marker-red.png',
    shadowUrl: 'img/marker-shadow.png',
    iconSize: [25, 41],
    iconAnchor: [12, 41],
    popupAnchor: [1, -34],
    shadowSize: [41, 41]
});

function createPathView(path, distance, distanceType, elevation, color) {
    // create [lat, lng] array for leaflet map
    let coords = path.map(node => [node.latitude, node.longitude]);
    let offTrackStart = L.polyline([startPoint, coords[0]], {
        'dashArray': 10,
        'weight': 2
    });
    let edge = L.polyline(coords);
    edge.setStyle({
        color: color
    });
    let offTrackEnd = L.polyline([coords[coords.length - 1], endPoint], {
        'dashArray': 10,
        'weight': 2
    });
    let newPath = L.layerGroup([offTrackStart, edge, offTrackEnd]);

    pathsOnMap.push(newPath);
    map.addLayer(newPath);
    map.fitBounds([startPoint, endPoint]);

    edge.bindPopup(`length: ${distance}${distanceType}\n` +
        `elevation: ${elevation.toFixed(2)}'m'`);
    edge.on('mouseover', function (e) {
        this.openPopup();
        showElevationGraph(path.map(node => node.elevation), elevation, color);
    });
    edge.on('mouseout', function (e) {
        this.closePopup();
    });

    return newPath;
}