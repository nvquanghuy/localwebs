let autoRefreshInterval;
let currentView = localStorage.getItem('localwebs-view') || 'grid';
let currentServices = [];
let sortBy = localStorage.getItem('localwebs-sortBy') || 'starttime';
let sortAscending = localStorage.getItem('localwebs-sortAscending') !== 'true'; // Default false for starttime (newest first)

// Initialize UI from saved preferences
function initializeUI() {
    // Set view toggle button text
    const toggleBtn = document.getElementById('view-toggle-btn');
    toggleBtn.textContent = currentView === 'grid' ? '📋 List View' : '🎴 Grid View';

    // Set sort dropdown
    const sortSelect = document.getElementById('sort-select');
    sortSelect.value = sortBy;

    // Set sort order button
    const sortOrderBtn = document.getElementById('sort-order-btn');
    sortOrderBtn.textContent = sortAscending ? '↑' : '↓';
}

async function fetchServices() {
    try {
        const response = await fetch('/api/services');
        const services = await response.json();
        currentServices = services;
        displayServices(services);
    } catch (error) {
        console.error('Failed to fetch services:', error);
        showError();
    }
}

function displayServices(services) {
    const grid = document.getElementById('services-grid');
    const list = document.getElementById('services-list');
    const loading = document.getElementById('loading');
    const emptyState = document.getElementById('empty-state');
    const controls = document.getElementById('controls');
    const serviceCount = document.getElementById('service-count');

    loading.style.display = 'none';

    if (services.length === 0) {
        grid.innerHTML = '';
        list.innerHTML = '';
        emptyState.style.display = 'block';
        controls.style.display = 'none';
        return;
    }

    emptyState.style.display = 'none';
    controls.style.display = 'flex';
    serviceCount.textContent = services.length;

    // Sort services
    const sortedServices = sortServices(services);

    if (currentView === 'grid') {
        displayGridView(sortedServices);
    } else {
        displayListView(sortedServices);
    }
}

function sortServices(services) {
    const sorted = [...services];

    sorted.sort((a, b) => {
        let compareValue = 0;

        switch (sortBy) {
            case 'starttime':
                const aStart = a.start_time || 0;
                const bStart = b.start_time || 0;
                compareValue = bStart - aStart; // Newer first by default
                break;
            case 'port':
                compareValue = a.port - b.port;
                break;
            case 'name':
                compareValue = a.name.localeCompare(b.name);
                break;
            case 'status':
                compareValue = (b.is_healthy ? 1 : 0) - (a.is_healthy ? 1 : 0);
                break;
            case 'response':
                const aTime = a.response_time_ms || 9999;
                const bTime = b.response_time_ms || 9999;
                compareValue = aTime - bTime;
                break;
        }

        return sortAscending ? compareValue : -compareValue;
    });

    return sorted;
}

function displayGridView(services) {
    const grid = document.getElementById('services-grid');
    const list = document.getElementById('services-list');

    grid.style.display = 'grid';
    list.style.display = 'none';

    grid.innerHTML = services.map(service => createServiceCard(service)).join('');

    // Add click listeners to cards
    document.querySelectorAll('.service-card').forEach((card, index) => {
        card.addEventListener('click', () => {
            const serviceUrl = buildServiceUrl(services[index].port);
            window.open(serviceUrl, '_blank');
        });
    });
}

function displayListView(services) {
    const grid = document.getElementById('services-grid');
    const list = document.getElementById('services-list');

    grid.style.display = 'none';
    list.style.display = 'block';

    list.innerHTML = `
        <table class="services-table">
            <thead>
                <tr>
                    <th>Status</th>
                    <th>Port</th>
                    <th>Name</th>
                    <th>Process</th>
                    <th>Response</th>
                    <th>Source</th>
                </tr>
            </thead>
            <tbody>
                ${services.map(service => createServiceRow(service)).join('')}
            </tbody>
        </table>
    `;

    // Add click listeners to rows
    document.querySelectorAll('.service-row').forEach((row, index) => {
        row.addEventListener('click', () => {
            const serviceUrl = buildServiceUrl(services[index].port);
            window.open(serviceUrl, '_blank');
        });
    });
}

function createServiceCard(service) {
    const statusClass = service.is_healthy ? 'status-healthy' : 'status-unhealthy';
    const responseTime = service.response_time_ms
        ? `${service.response_time_ms}ms`
        : 'N/A';

    const processInfo = service.process_name
        ? `<div class="info-row">
               <span class="info-label">Process:</span>
               <span>${service.process_name}${service.pid ? ` (${service.pid})` : ''}</span>
           </div>`
        : '';

    const title = service.title
        ? `<div class="info-row">
               <span class="info-label">Title:</span>
               <span>${escapeHtml(service.title)}</span>
           </div>`
        : '';

    const description = service.description
        ? `<div class="info-row">
               <span class="info-label">Description:</span>
               <span>${escapeHtml(service.description)}</span>
           </div>`
        : '';

    return `
        <div class="service-card">
            <div class="service-header">
                <div class="service-name">${escapeHtml(service.name)}</div>
                <div class="status-indicator ${statusClass}"></div>
            </div>
            <div class="service-port">:${service.port}</div>
            <div class="service-info">
                ${description}
                ${processInfo}
                ${title}
                <div class="info-row">
                    <span class="info-label">Response:</span>
                    <span>${responseTime}</span>
                </div>
                <div class="info-row">
                    <span class="info-label">Source:</span>
                    <span class="source-badge">${service.source}</span>
                </div>
            </div>
        </div>
    `;
}

function formatStartTime(timestamp) {
    if (!timestamp) return '-';

    const now = Math.floor(Date.now() / 1000);
    const elapsed = now - timestamp;

    if (elapsed < 60) return 'just now';
    if (elapsed < 3600) return `${Math.floor(elapsed / 60)}m ago`;
    if (elapsed < 86400) return `${Math.floor(elapsed / 3600)}h ago`;
    return `${Math.floor(elapsed / 86400)}d ago`;
}

function createServiceRow(service) {
    const statusIcon = service.is_healthy ? '🟢' : '🔴';
    const responseTime = service.response_time_ms
        ? `${service.response_time_ms}ms`
        : 'N/A';
    const processInfo = service.process_name
        ? `${service.process_name}${service.pid ? ` (${service.pid})` : ''}`
        : '-';
    const startTime = formatStartTime(service.start_time);

    return `
        <tr class="service-row">
            <td class="status-cell">${statusIcon}</td>
            <td class="port-cell"><strong>:${service.port}</strong></td>
            <td class="name-cell">
                <div class="name-primary">${escapeHtml(service.name)}</div>
                ${service.title ? `<div class="name-secondary">${escapeHtml(service.title)}</div>` : ''}
                ${service.start_time ? `<div class="name-tertiary">Started ${startTime}</div>` : ''}
            </td>
            <td class="process-cell">${processInfo}</td>
            <td class="response-cell">${responseTime}</td>
            <td class="source-cell"><span class="source-badge-small">${service.source}</span></td>
        </tr>
    `;
}

function buildServiceUrl(port) {
    const hostname = window.location.hostname;
    return `http://${hostname}:${port}`;
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function showError() {
    const loading = document.getElementById('loading');
    loading.innerHTML = '<p style="color: white;">Failed to load services. Please try again.</p>';
}

function toggleView() {
    currentView = currentView === 'grid' ? 'list' : 'grid';
    const toggleBtn = document.getElementById('view-toggle-btn');
    toggleBtn.textContent = currentView === 'grid' ? '📋 List View' : '🎴 Grid View';

    // Save preference
    localStorage.setItem('localwebs-view', currentView);

    displayServices(currentServices);
}

function toggleSortOrder() {
    sortAscending = !sortAscending;
    const sortOrderBtn = document.getElementById('sort-order-btn');
    sortOrderBtn.textContent = sortAscending ? '↑' : '↓';

    // Save preference
    localStorage.setItem('localwebs-sortAscending', sortAscending);

    displayServices(currentServices);
}

function changeSortBy(value) {
    sortBy = value;

    // Save preference
    localStorage.setItem('localwebs-sortBy', sortBy);

    displayServices(currentServices);
}

function startAutoRefresh() {
    autoRefreshInterval = setInterval(fetchServices, 10000);
}

function stopAutoRefresh() {
    if (autoRefreshInterval) {
        clearInterval(autoRefreshInterval);
    }
}

// Event listeners
document.getElementById('refresh-btn').addEventListener('click', async () => {
    document.getElementById('loading').style.display = 'block';
    document.getElementById('services-grid').innerHTML = '';
    document.getElementById('services-list').innerHTML = '';
    await fetchServices();
});

document.getElementById('view-toggle-btn').addEventListener('click', toggleView);
document.getElementById('sort-select').addEventListener('change', (e) => changeSortBy(e.target.value));
document.getElementById('sort-order-btn').addEventListener('click', toggleSortOrder);

// Initialize UI with saved preferences
initializeUI();

// Initial load
fetchServices();

// Start auto-refresh
startAutoRefresh();

// Stop auto-refresh when page is hidden
document.addEventListener('visibilitychange', () => {
    if (document.hidden) {
        stopAutoRefresh();
    } else {
        startAutoRefresh();
    }
});
