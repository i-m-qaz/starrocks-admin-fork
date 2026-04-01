-- ========================================
-- StarRocks Admin - Add Database and Table Management Permissions
-- ========================================
-- Created: 2026-04-01
-- Purpose: Add permissions for database and table management functionality

-- ==============================================
-- 1. Insert Menu Permissions for DB Management
-- ==============================================
INSERT OR IGNORE INTO permissions (code, name, type, resource, action, description) VALUES
('menu:db-management', '库表管理', 'menu', 'db-management', 'view', '查看库表管理'),
('menu:db-management:databases', '数据库管理', 'menu', 'db-management', 'view', '查看数据库管理'),
('menu:db-management:tables', '表管理', 'menu', 'db-management', 'view', '查看表管理');

-- ==============================================
-- 2. Insert API Permissions for Database Management
-- ==============================================
INSERT OR IGNORE INTO permissions (code, name, type, resource, action, description) VALUES
-- Database Management
('api:clusters:databases:manage', '管理数据库', 'api', 'clusters', 'databases:manage', '管理数据库'),
('api:clusters:databases:create', '创建数据库', 'api', 'clusters', 'databases:create', '创建数据库'),
('api:clusters:databases:get', '获取数据库详情', 'api', 'clusters', 'databases:get', '获取数据库详情'),
('api:clusters:databases:update', '更新数据库', 'api', 'clusters', 'databases:update', '更新数据库'),
('api:clusters:databases:delete', '删除数据库', 'api', 'clusters', 'databases:delete', '删除数据库');

-- ==============================================
-- 3. Insert API Permissions for Table Management
-- ==============================================
INSERT OR IGNORE INTO permissions (code, name, type, resource, action, description) VALUES
-- Table Management
('api:clusters:tables:manage', '管理表', 'api', 'clusters', 'tables:manage', '管理表'),
('api:clusters:tables:create', '创建表', 'api', 'clusters', 'tables:create', '创建表'),
('api:clusters:tables:get', '获取表详情', 'api', 'clusters', 'tables:get', '获取表详情'),
('api:clusters:tables:update', '更新表', 'api', 'clusters', 'tables:update', '更新表'),
('api:clusters:tables:delete', '删除表', 'api', 'clusters', 'tables:delete', '删除表'),
('api:clusters:tables:partition', '管理表分区', 'api', 'clusters', 'tables:partition', '管理表分区'),
('api:clusters:tables:bucket', '管理表分桶', 'api', 'clusters', 'tables:bucket', '管理表分桶'),
('api:clusters:tables:action', '执行表操作', 'api', 'clusters', 'tables:action', '执行表操作');

-- ==============================================
-- 4. Set Parent-Child Relationships
-- ==============================================
-- Set parent_id for database management menu
UPDATE permissions 
SET parent_id = (SELECT id FROM permissions WHERE code = 'menu:db-management')
WHERE code = 'menu:db-management:databases';

-- Set parent_id for table management menu
UPDATE permissions 
SET parent_id = (SELECT id FROM permissions WHERE code = 'menu:db-management')
WHERE code = 'menu:db-management:tables';

-- Set parent_id for database management APIs
UPDATE permissions 
SET parent_id = (SELECT id FROM permissions WHERE code = 'menu:db-management:databases')
WHERE code IN (
    'api:clusters:databases:manage',
    'api:clusters:databases:create',
    'api:clusters:databases:get',
    'api:clusters:databases:update',
    'api:clusters:databases:delete'
);

-- Set parent_id for table management APIs
UPDATE permissions 
SET parent_id = (SELECT id FROM permissions WHERE code = 'menu:db-management:tables')
WHERE code IN (
    'api:clusters:tables:manage',
    'api:clusters:tables:create',
    'api:clusters:tables:get',
    'api:clusters:tables:update',
    'api:clusters:tables:delete',
    'api:clusters:tables:partition',
    'api:clusters:tables:bucket',
    'api:clusters:tables:action'
);

-- ==============================================
-- 5. Assign All New Permissions to Admin Role
-- ==============================================
-- Admin role gets ALL permissions
INSERT OR IGNORE INTO role_permissions (role_id, permission_id)
SELECT (SELECT id FROM roles WHERE code='admin'), id FROM permissions
WHERE code LIKE 'menu:db-management%' OR code LIKE 'api:clusters:databases%' OR code LIKE 'api:clusters:tables%';

-- ========================================
-- MIGRATION COMPLETE
-- ========================================
-- New Permissions Added:
--   - 3 menu permissions (db-management, databases, tables)
--   - 5 database management API permissions
--   - 8 table management API permissions
-- All permissions assigned to admin role
--
-- Permission Coverage:
--   ✓ Database CRUD operations
--   ✓ Table CRUD operations
--   ✓ Table partition management
--   ✓ Table bucket management
--   ✓ Table actions (truncate, optimize, etc.)