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
let lastPaths = [];
let chart;
let xhr = new XMLHttpRequest();

let EDGE_COLORS = ['black','red','green','blue','orange','yellow', 'purple', 'pink', 'gold', 'tomato', 'olivedrab'];
let edge_color_count = 0;

function get_next_edge_color() {
    // looping over edge colors
    return EDGE_COLORS[edge_color_count++ % EDGE_COLORS.length]
}

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
    if (lastPaths.length > 0) {
        for (let path of lastPaths) {
            map.removeLayer(path);
        }
        lastPaths = []
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
    if (lastPaths.length > 0) {
        for (let path of lastPaths) {
            map.removeLayer(path);
        }
        lastPaths = []
    }
}

function showElevationGraph(elevationPoints, totalElevation, color) {
    console.log(elevationPoints);
    let result = document.getElementById('totalElevation');
    result.innerText = 'Total: ' + totalElevation.toFixed(2) + 'm';
    let graph = document.getElementById('elevationGraph').getContext('2d');
    if (chart) {
        chart.destroy()
    }
    chart = new Chart(graph, {
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
    graphContainer = document.getElementById("graphContainer");
    graphContainer.style.display = "block";
}

//TODO: First create paths and add to map, keep references, then create total result, compare paths, ...
function query() {
    hideResult();
    hideInvalidRequest();
    hideNoPathFound();
    hideSelectStartAndEnd();

    if (lastPaths.length > 1) {
        for (let path of lastPaths) {
            map.removeLayer(path);
        }
        lastPaths = []
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
            let json = JSON.parse(xhr.responseText);
            // result is ordered by descending length
            showResultToast(json[json.length-1].cost);
            for (result of json) {
                console.log('single result', result);
                if (result.path) {
                    console.log(result.path);
                    addResult(result);
                } else {
                    showNoPathFound();
                }
            }
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

function addResult(result) {
    let totalEle = result.path.map(node => node.elevation).reduce((total, currentValue, currentIndex, elevation) => {
        if (currentIndex === 0) {
            return 0
        }
        let delta = currentValue - elevation[currentIndex - 1];
        if (delta > 0) {
            return total + delta
        } else {
            return total
        }
    }, 0);

    createPathView(result.path, result.cost, totalEle);
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

function showResultToast(costs) {
    var tmp = document.getElementById("result");
    tmp.innerHTML = `Shortest path has ${costs}`;
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

function createPathView(path, cost, elevation) {
    // create [lat, lng] array for leaflet map
    let coords = path.map(node => [node.latitude, node.longitude]);
    offTrackStart = L.polyline([startPoint, coords[0]], {
        'dashArray': 10,
        'weight': 2
    });
    let edge = L.polyline(coords);
    let color = get_next_edge_color()
    edge.setStyle({
        color: color
    });
    let offTrackEnd = L.polyline([coords[coords.length - 1], endPoint], {
        'dashArray': 10,
        'weight': 2
    });
    let newPath = L.layerGroup([offTrackStart, edge, offTrackEnd])

    lastPaths.push(newPath);
    map.addLayer(newPath);
    map.fitBounds([startPoint, endPoint]);

    edge.bindPopup(`length: ${cost}\n` +
        `elevation: ${elevation.toFixed(2) + 'm'}`);
    edge.on('mouseover', function (e) {
        this.openPopup();
        console.log('path', path);
        showElevationGraph(path.map(node => node.elevation), elevation, color);
    });
    edge.on('mouseout', function (e) {
        this.closePopup();
    })
}